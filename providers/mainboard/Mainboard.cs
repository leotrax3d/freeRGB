// freeRGB optional motherboard reader. No control setters or driver installation.
// Compiles with the Windows .NET Framework compiler; LHM is loaded only on request.
using System;
using System.Collections;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Security.Principal;
using System.ServiceProcess;
using System.Text;
using System.Web.Script.Serialization;

internal static class Mainboard
{
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = 262144 };
    private static object computer;
    private static string library;
    private static bool direct;
#if BROKER_FIXTURE_CLIENT_AUTH
    private static bool fixtureAuthStarted;
    private static volatile bool fixtureClientAuthenticated;
#if BROKER_FIXTURE_WORKER
    private static bool fixtureJournalVerified;
#endif
    private static void StartFixtureAuth()
    {
        if (fixtureAuthStarted) return;
        fixtureAuthStarted = true;
        var pipe = BrokerTransport.CreateServer("freergb-native-auth-fixture-" + Process.GetCurrentProcess().Id,
            new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null));
        var thread = new System.Threading.Thread(delegate() {
            using (pipe)
            {
                try
                {
                    pipe.WaitForConnection();
                    fixtureClientAuthenticated = BrokerTransport.IsServiceClient(pipe) && BrokerTransport.ReadSnapshotRequest(pipe);
                    BrokerTransport.Write(pipe, Encoding.ASCII.GetBytes(fixtureClientAuthenticated ? "authenticated\n" : "denied\n"));
                    BrokerTransport.ReadAcknowledgment(pipe);
                }
                catch { fixtureClientAuthenticated = false; }
            }
        }) { IsBackground = true };
        thread.Start();
    }
#endif

    private static object Get(object target, string name)
    {
        return target.GetType().GetProperty(name).GetValue(target, null);
    }
    private static void Invoke(object target, string name)
    {
        target.GetType().GetMethod(name, Type.EmptyTypes).Invoke(target, null);
    }
    private static Dictionary<string, object> Result(string status, string message, object hardware)
    {
        return new Dictionary<string, object> { {"version", 1}, {"status", status}, {"message", message}, {"hardware", hardware} };
    }
    private static Dictionary<string, object> Empty(string status, string message)
    {
        return Result(status, message, new object[0]);
    }
    private static Dictionary<string, object> Initialize()
    {
        string dll = Path.Combine(library, "LibreHardwareMonitorLib.dll");
        if (!File.Exists(dll)) return Empty("missing_library", "LibreHardwareMonitor 0.9.6 library and its runtime dependencies are required.");
        // Never start/install a driver or ask for elevation from the helper.
        bool running = false;
        using (var driver = new ServiceController("PawnIO"))
        {
            try { running = driver.Status == ServiceControllerStatus.Running; }
            catch (InvalidOperationException) { }
        }
        if (!running) return Empty("missing_driver", "PawnIO is not running. Use freeRGB Setup's optional mainboard components or check the existing driver installation.");
        using (WindowsIdentity identity = WindowsIdentity.GetCurrent())
        {
            if (!new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator))
                return Empty("permission_denied", "Motherboard access requires the protected freeRGB mainboard broker. No GUI/daemon elevation was requested automatically.");
        }
        AppDomain.CurrentDomain.AssemblyResolve += delegate(object sender, ResolveEventArgs args)
        {
            string name = new AssemblyName(args.Name).Name;
            if (name.IndexOfAny(new char[] {'/', '\\', ':'}) >= 0) return null;
            string dependency = Path.Combine(library, name + ".dll");
            return File.Exists(dependency) ? Assembly.LoadFrom(dependency) : null;
        };
        Assembly assembly = Assembly.LoadFrom(dll);
        Version version = assembly.GetName().Version;
        if (version.Major != 0 || version.Minor != 9 || version.Build != 6)
            return Empty("missing_library", "This provider requires the verified LibreHardwareMonitor 0.9.6 runtime.");
        computer = Activator.CreateInstance(assembly.GetType("LibreHardwareMonitor.Hardware.Computer", true));
        // Default Computer settings are memory-only. No persisted control settings
        // are loaded. Only motherboard/SuperIO groups are enabled, never USB/GPU.
        computer.GetType().GetProperty("IsMotherboardEnabled").SetValue(computer, true, null);
        Invoke(computer, "Open");
        return null;
    }
    private static void Walk(object hardware, List<object> output, Stopwatch clock, int depth, ref int sensorCount)
    {
        if (depth > 16 || output.Count >= 64) throw new InvalidDataException("Hardware tree limit exceeded.");
        Invoke(hardware, "Update");
        var sensors = new List<object>();
        foreach (object sensor in (IEnumerable)Get(hardware, "Sensors"))
        {
            string type = Get(sensor, "SensorType").ToString();
            string kind = type == "Temperature" ? "temperature" : type == "Fan" ? "fan" : type == "Control" ? "control" : null;
            if (kind == null) continue;
            if (++sensorCount > 1024) throw new InvalidDataException("Sensor count limit exceeded.");
            object raw = Get(sensor, "Value");
            object value = null;
            if (raw != null)
            {
                double number = Convert.ToDouble(raw, System.Globalization.CultureInfo.InvariantCulture);
                if (!Double.IsNaN(number) && !Double.IsInfinity(number)) value = number;
            }
            sensors.Add(new Dictionary<string, object> {
                {"id", Get(sensor, "Identifier").ToString()}, {"name", Get(sensor, "Name").ToString()},
                {"kind", kind}, {"value", value}, {"age_ms", clock.ElapsedMilliseconds},
                {"control_available", kind == "control" && Get(sensor, "Control") != null}
            });
        }
        output.Add(new Dictionary<string, object> { {"id", Get(hardware, "Identifier").ToString()}, {"name", Get(hardware, "Name").ToString()}, {"sensors", sensors} });
        foreach (object child in (IEnumerable)Get(hardware, "SubHardware")) Walk(child, output, clock, depth + 1, ref sensorCount);
    }
    private static object Snapshot()
    {
#if BROKER_FIXTURE_WORKER
        // Compile-time CI substitute only; production builds have no fixture switch.
        if (!direct) return Empty("faulted", "CI fixture has no direct fallback; the protected broker must answer.");
#if BROKER_FIXTURE_CLIENT_AUTH
        StartFixtureAuth();
        if (!fixtureJournalVerified)
        {
            using (var journal = NativeControlJournal.CreateProtected())
            {
                string id = journal.Reserve(new NativeControlIdentity("/fixture/board","/fixture/control/7",false));
                if ((string)journal.Pending()["id"] != id) throw new InvalidOperationException("Protected journal persistence failed.");
                journal.Complete(id);
                if (journal.Pending() != null) throw new InvalidOperationException("Protected journal clearing failed.");
            }
            fixtureJournalVerified = true;
        }
        string fixtureMessage = fixtureClientAuthenticated ? "service_client_authenticated;journal_verified" : "service_client_auth_pending";
#else
        string fixtureMessage = "Isolated SCM fixture; no driver or hardware accessed.";
#endif
#if BROKER_FIXTURE_V2
        int rpm = 901;
#else
        int rpm = 900;
#endif
        return Result("ready", fixtureMessage, new object[] {
            new { id = "fixture", name = "Fixture", sensors = new object[] {
                new { id = "rpm", name = "Fixture RPM", kind = "fan", value = rpm, age_ms = 0, control_available = false }
            } }
        });
#else
        if (computer == null)
        {
            var unavailable = Initialize();
            if (unavailable != null) return unavailable;
        }
        var hardware = new List<object>();
        var clock = Stopwatch.StartNew();
        int count = 0;
        foreach (object item in (IEnumerable)Get(computer, "Hardware")) Walk(item, hardware, clock, 0, ref count);
        if (count == 0) return Empty("no_supported_hardware", "No supported motherboard sensor channels were reported; this does not prove no fans are connected.");
        return Result("ready", "Read-only motherboard channels. Fan/header pairing, PWM/DC mode and physical labels are unconfirmed; RGB uses a separate provider.", hardware);
#endif
    }
    private static int Main(string[] args)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        if (args.Length == 1 && args[0] == "--self-test")
        {
            // Tests serialization without loading LHM, touching hardware or drivers.
            Console.WriteLine(Json.Serialize(Empty("missing_library", "Self-test only: no hardware accessed.")));
            return 0;
        }
        if (args.Length == 1 && args[0] == "--direct-protected-library")
        {
            // Only the protected LocalSystem service worker may bypass the broker.
            try { BrokerService.CheckInstallation(true); }
            catch { return 3; }
            library = BrokerService.RuntimePath;
            direct = true;
        }
        else
        {
            if (args.Length != 2 || args[0] != "--library") return 2;
            // Rust canonical paths use the Win32 extended prefix. Framework path
            // validation rejects it unless legacy long-path switches are changed.
            string directory = args[1];
            if (directory.StartsWith(@"\\?\UNC\", StringComparison.OrdinalIgnoreCase)) directory = @"\\" + directory.Substring(8);
            else if (directory.StartsWith(@"\\?\", StringComparison.Ordinal)) directory = directory.Substring(4);
            library = Path.GetFullPath(directory);
        }
        try
        {
            string request;
            while ((request = Console.ReadLine()) != null)
            {
                if (request != "snapshot") return 2;
                string json;
                try
                {
                    string broker = null;
                    if (!direct)
                    {
                        try { broker = BrokerTransport.Snapshot(); }
                        catch (Win32Exception error) { if (error.NativeErrorCode != 2) throw; }
                    }
                    json = broker ?? Json.Serialize(Snapshot());
                }
                catch (Exception error)
                {
                    Exception cause = error is TargetInvocationException && error.InnerException != null ? error.InnerException : error;
                    string detail = cause is InvalidDataException ? " " + cause.Message : "";
                    json = Json.Serialize(Empty("faulted", "Motherboard provider failed (" + cause.GetType().Name + ")." + detail + " Verify library dependencies, protected broker access and conflicting monitoring software."));
                }
                if (Encoding.UTF8.GetByteCount(json) >= 262144) json = Json.Serialize(Empty("faulted", "Motherboard snapshot exceeds the message limit."));
                Console.WriteLine(json);
                Console.Out.Flush();
            }
        }
        finally
        {
            // Stock SuperIOHardware.Close() calls SetControl(index, null), even
            // for a reader. We never acquire software fan control, so do not
            // run that cleanup path. This dedicated process exits immediately;
            // Windows reclaims its handles and memory. Do not reuse this helper
            // as an in-process library or add fan writes without a new lifecycle.
            computer = null;
        }
        return 0;
    }
}
