// Authenticated service -> protected native worker pipe, including the fixed100
// commissioning worker. No direct GUI/native access.
using System;
using System.IO;
using System.IO.Pipes;
using System.Security.Principal;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

internal static class NativeSessionTransport
{
#if NATIVE_CONTROL_TESTS
    internal static void RunForTest(NamedPipeServerStream pipe, NativeControlSession session) { Run(pipe, session); }
#else
    internal static void RunProtected(NamedPipeServerStream pipe, JournaledNativeControl port, NativeTelemetryGuard telemetry, Func<long> clock)
    {
        BrokerService.CheckInstallation(true);
        if (pipe == null || !pipe.IsConnected) throw new InvalidOperationException("Connected protected worker pipe required.");
        var owner = (SecurityIdentifier)pipe.GetAccessControl().GetOwner(typeof(SecurityIdentifier));
        if (!owner.IsWellKnown(WellKnownSidType.LocalSystemSid) || !BrokerTransport.IsServiceClient(pipe))
            throw new InvalidOperationException("Native session client is not the registered protected service.");
        // No control acquisition before authenticating the peer.
        Run(pipe, NativeControlSession.CreateProtected(port, telemetry, clock));
    }
    internal static void RunTieredProtected(NamedPipeServerStream pipe, JournaledNativeControl port, NativeTelemetryGuard telemetry, Func<long> clock, NativeTier tier)
    {
        BrokerService.CheckInstallation(true);
        if (pipe == null || !pipe.IsConnected) throw new InvalidOperationException("Connected protected worker pipe required.");
        var owner = (SecurityIdentifier)pipe.GetAccessControl().GetOwner(typeof(SecurityIdentifier));
        if (!owner.IsWellKnown(WellKnownSidType.LocalSystemSid) || !BrokerTransport.IsServiceClient(pipe))
            throw new InvalidOperationException("Native session client is not the registered protected service.");
        Run(pipe, NativeControlSession.CreateTieredProtected(port, telemetry, clock, tier));
    }
#endif
    private static void Write(NamedPipeServerStream pipe, NativeControlSession session)
    {
        string record = new JavaScriptSerializer { MaxJsonLength = 2048 }.Serialize(session.View());
        BrokerTransport.Write(pipe, Encoding.UTF8.GetBytes(record + "\n"));
    }
    private static void Run(NamedPipeServerStream pipe, NativeControlSession session)
    {
        using (var stop = new ManualResetEvent(false))
        {
            var watchdog = new Thread(delegate() {
                try { while (!stop.WaitOne(25) && session.Poll()) { } }
                catch { session.Fault("watchdog_failed"); }
            }) { IsBackground = true, Priority = ThreadPriority.AboveNormal, Name = "freeRGB native session lease" };
            watchdog.Start();
            try
            {
                Write(pipe, session);
                while (session.Poll())
                {
                    string request = BrokerTransport.ReadRecord(pipe,2000,2048);
                    session.Execute(request);
                    Write(pipe, session);
                }
            }
            catch (Exception error)
            {
                if (error is OutOfMemoryException) throw;
                session.Fault(error is EndOfStreamException ? "peer_disconnected" : "transport_failed");
            }
            finally
            {
                session.Fault("transport_stopped");
                stop.Set();
                // Driver/native calls can still block: no claim that a thread
                // join or object lock is an independent driver-recovery path.
                if (!watchdog.Join(1500)) throw new InvalidOperationException("Native watchdog did not finish; external recovery required.");
            }
        }
    }
}
