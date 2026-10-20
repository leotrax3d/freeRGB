// Uses the real pinned LHM Control class with remoting interface stubs and fake
// event callbacks. Never creates/opens Computer, SuperIO, a service or a driver.
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.Remoting.Messaging;
using System.Runtime.Remoting.Proxies;

internal sealed class InterfaceStub : RealProxy
{
    private readonly Func<IMethodCallMessage, object> invoke;
    private readonly Type type;
    internal InterfaceStub(Type type, Func<IMethodCallMessage, object> invoke) : base(type) { this.type = type; this.invoke = invoke; }
    public override IMessage Invoke(IMessage message)
    {
        var call = (IMethodCallMessage)message;
        try { return new ReturnMessage(call.MethodName == "GetType" ? type : invoke(call), null, 0, call.LogicalCallContext, call); }
        catch (Exception error) { return new ReturnMessage(error, call); }
    }
}
internal sealed class ControlSink
{
    internal readonly List<float?> Writes = new List<float?>();
    internal bool FailSoftware, FailDefault;
    internal void Changed(object control)
    {
        Type type = control.GetType();
        int mode = Convert.ToInt32(type.GetProperty("ControlMode").GetValue(control, null));
        if (mode == 1)
        {
            if (FailSoftware) throw new IOException("Synthetic failed software write.");
            Writes.Add((float)type.GetProperty("SoftwareValue").GetValue(control, null));
        }
        else if (mode == 2)
        {
            if (FailDefault) throw new IOException("Synthetic failed firmware restore.");
            Writes.Add(null);
        }
    }
}
internal static class NativeControlTests
{
    private static Assembly library;
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action, string message)
    {
        try { action(); } catch (InvalidOperationException) { return; } catch (ArgumentException) { return; } catch (AggregateException) { return; } catch (TargetInvocationException) { return; }
        throw new Exception(message);
    }
    private static object Create(ControlSink sink)
    {
        Type identifier = library.GetType("LibreHardwareMonitor.Hardware.Identifier", true);
        object id = Activator.CreateInstance(identifier, new object[] { new[] { "synthetic", "control", "7" } });
        object sensor = new InterfaceStub(library.GetType("LibreHardwareMonitor.Hardware.ISensor", true), delegate(IMethodCallMessage call) {
            if (call.MethodName == "get_Identifier") return id;
            throw new InvalidOperationException("Unexpected sensor operation: " + call.MethodName);
        }).GetTransparentProxy();
        object settings = new InterfaceStub(library.GetType("LibreHardwareMonitor.Hardware.ISettings", true), delegate(IMethodCallMessage call) {
            if (call.MethodName == "GetValue") return call.Args[1];
            if (call.MethodName == "SetValue") return null;
            throw new InvalidOperationException("Unexpected settings operation: " + call.MethodName);
        }).GetTransparentProxy();
        Type type = library.GetType("LibreHardwareMonitor.Hardware.Control", true);
        object control = Activator.CreateInstance(type, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic, null, new object[] { sensor, settings, 0f, 100f }, null);
        foreach (string name in new[] { "ControlModeChanged", "SoftwareControlValueChanged" })
        {
            EventInfo change = type.GetEvent(name, BindingFlags.Instance | BindingFlags.NonPublic);
            Delegate callback = Delegate.CreateDelegate(change.EventHandlerType, sink, typeof(ControlSink).GetMethod("Changed", BindingFlags.Instance | BindingFlags.NonPublic));
            change.GetAddMethod(true).Invoke(control, new object[] { callback });
        }
        return control;
    }
    private static void Run()
    {
        var baseline = new ControlSink();
        object unsafeControl = Create(baseline);
        unsafeControl.GetType().GetMethod("SetSoftware").Invoke(unsafeControl, new object[] { 100f });
        Assert(baseline.Writes.Count == 2 && baseline.Writes[0] == 0 && baseline.Writes[1] == 100, "Pinned-library transition evidence changed; re-audit adapter.");

        var sink = new ControlSink();
        object control = Create(sink);
        Reject(delegate { NativeControl.ForTest(new object(), false); }, "Unexpected native object type accepted.");
        var port = NativeControl.ForTest(control, false);
        Reject(delegate { port.SetDuty(40); }, "Write without acquisition accepted.");
        port.Acquire();
        Assert(port.Owned && sink.Writes.Count == 1 && sink.Writes[0] == 100, "Acquisition emitted an unprepared intermediate duty.");
        var second = NativeControl.ForTest(control, false);
        Reject(second.Acquire, "Second owner acquired the same control.");
        port.SetDuty(40);
        port.SetDuty(40);
        Assert(sink.Writes.Count == 3 && sink.Writes[1] == 40 && sink.Writes[2] == 40, "Repeated duty was not actually reasserted.");
        foreach (float bad in new[] { 0f,39f,101f,Single.NaN,Single.PositiveInfinity }) Reject(delegate { port.SetDuty(bad); }, "Invalid duty reached native state.");
        Assert(sink.Writes.Count == 3, "Invalid duty produced I/O.");
        port.Release(); port.Release();
        Assert(!port.Owned && sink.Writes.Count == 4 && sink.Writes[3] == null, "Owned firmware restoration or idempotency failed.");
        second.Acquire(); second.Release();
        Reject(delegate { NativeControl.ForTest(unsafeControl,false).Acquire(); }, "Foreign Software mode was overwritten.");
        Assert(baseline.Writes.Count == 2, "Foreign control rejection performed I/O.");

        var lostSink = new ControlSink();
        object lostControl = Create(lostSink);
        var lost = NativeControl.ForTest(lostControl, false);
        lost.Acquire();
        lostControl.GetType().GetMethod("SetDefault").Invoke(lostControl, null);
        Reject(delegate { lost.SetDuty(60); }, "Lost software mode was overwritten without explicit recovery.");
        Assert(lost.Owned && lostSink.Writes.Count == 2, "Lost-mode rejection performed I/O or discarded recovery ownership.");
        lost.Release();
        Assert(!lost.Owned && lostSink.Writes.Count == 3 && lostSink.Writes[2] == null, "Explicit lost-mode recovery did not retry firmware restoration.");

        var pumpSink = new ControlSink();
        var pump = NativeControl.ForTest(Create(pumpSink),true);
        pump.Acquire();
        Reject(delegate { pump.SetDuty(99); }, "Pump reduction accepted.");
        pump.Release();
        Assert(pumpSink.Writes.Count == 2 && pumpSink.Writes[0] == 100 && pumpSink.Writes[1] == null, "Pump received a reduced duty.");

        var failedSink = new ControlSink { FailSoftware = true, FailDefault = true };
        object failedControl = Create(failedSink);
        var failed = NativeControl.ForTest(failedControl,false);
        Reject(failed.Acquire,"Failed native acquisition was reported successful.");
        Assert(failed.Owned,"Partial acquisition lost recovery ownership.");
        Reject(delegate { NativeControl.ForTest(failedControl,false).Acquire(); }, "Failed recovery released the reservation.");
        failedSink.FailSoftware = false; failedSink.FailDefault = false;
        failed.Release();
        Assert(!failed.Owned && failedSink.Writes.Count == 1 && failedSink.Writes[0] == null,"Retry skipped the cached Default callback after failed restoration.");

        var writeSink = new ControlSink();
        var writer = NativeControl.ForTest(Create(writeSink),false);
        writer.Acquire(); writeSink.FailSoftware = true;
        Reject(delegate { writer.SetDuty(60); },"Failed duty write was reported successful.");
        Assert(!writer.Owned && writeSink.Writes.Count == 2 && writeSink.Writes[1] == null,"Failed duty did not attempt owned firmware restoration.");

        var sessionSink = new ControlSink();
        var sessionPort = NativeControl.ForTest(Create(sessionSink),false);
        long now = 0;
        var session = NativeControlSession.ForTest(sessionPort,delegate { return now; });
        session.Execute(NativeSessionTests.Command(session.View(),"set",40));
        now = 1000;
        Assert(!session.Poll() && sessionPort.Owned && sessionSink.Writes.Count == 3 && sessionSink.Writes[0] == 100 && sessionSink.Writes[1] == 40 && sessionSink.Writes[2] == 100,"Session lease did not drive the real pinned Control adapter fallback correctly.");
        sessionPort.Release();
    }
    private static int Main(string[] args)
    {
        if (args.Length != 1) return 2;
        string directory = Path.GetFullPath(args[0]);
        AppDomain.CurrentDomain.AssemblyResolve += delegate(object sender, ResolveEventArgs request) {
            string name = new AssemblyName(request.Name).Name;
            if (name.IndexOfAny(new[] { '/', '\\', ':' }) >= 0) return null;
            string path = Path.Combine(directory,name + ".dll");
            return File.Exists(path) ? Assembly.LoadFrom(path) : null;
        };
        library = Assembly.LoadFrom(Path.Combine(directory,"LibreHardwareMonitorLib.dll"));
        Run();
        NativeSessionTests.Run();
        Console.WriteLine("PASS: real pinned LHM Control ordering, no-zero acquisition, ownership, pump limits, failed writes and repeated firmware recovery. Synthetic callbacks only; no hardware opened.");
        return 0;
    }
}
