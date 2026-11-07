// Optional reader and protected administrator-only fixed100 diagnostic.
// No public fan setter or driver installation; general control remains gated.
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
    private static Dictionary<string, object> Initialize(bool motherboardOnly = false)
    {
        string dll = Path.Combine(library, "LibreHardwareMonitorLib.dll");
        if (!File.Exists(dll)) return Empty("missing_library", "LibreHardwareMonitor " + LhmPin.Version + " library and its runtime dependencies are required.");
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
        if (version.Major != LhmPin.Assembly.Major || version.Minor != LhmPin.Assembly.Minor || version.Build != LhmPin.Assembly.Build)
            return Empty("missing_library", "This provider requires the verified LibreHardwareMonitor " + LhmPin.Version + " runtime.");
        computer = Activator.CreateInstance(assembly.GetType("LibreHardwareMonitor.Hardware.Computer", true));
        // Default Computer settings are memory-only. No persisted control settings
        // are loaded. Motherboard/SuperIO is the only control group; USB is never
        // enabled. Session control ids are restricted to /lpc/ (cooling_sessions.rs).
        computer.GetType().GetProperty("IsMotherboardEnabled").SetValue(computer, true, null);
        // CPU and GPU temperatures, read-only, as curve sources. Storage stays off:
        // LHM storage enumeration blocked the first snapshot on the owner board
        // (COOLING_ACCEPTANCE.md, 2026-11-05) and would starve fan telemetry.
        // Dedicated control workers must not open another CPU/GPU monitor. They
        // consume those temperatures from the authenticated reader cache instead.
        computer.GetType().GetProperty("IsCpuEnabled").SetValue(computer, !motherboardOnly, null);
        computer.GetType().GetProperty("IsGpuEnabled").SetValue(computer, !motherboardOnly, null);
        Invoke(computer, "Open");
        return null;
    }
    private static readonly Stopwatch Uptime = Stopwatch.StartNew();
    private static readonly UpdateCadence cadence = new UpdateCadence();
    private static readonly Dictionary<string, List<object>> reused = new Dictionary<string, List<object>>();
    private static bool IsMotherboard(object hardware)
    {
        return Get(hardware, "HardwareType").ToString() == "Motherboard";
    }
    // Copies cached hardware entries with every sensor aged to `age` ms.
    private static List<object> Aged(List<object> cached, long age, ref int sensorCount)
    {
        var copy = new List<object>();
        foreach (Dictionary<string, object> hardware in cached)
        {
            var sensors = new List<object>();
            foreach (Dictionary<string, object> sensor in (List<object>)hardware["sensors"])
            {
                if (++sensorCount > 1024) throw new InvalidDataException("Sensor count limit exceeded.");
                var entry = new Dictionary<string, object>(sensor);
                entry["age_ms"] = age;
                sensors.Add(entry);
            }
            copy.Add(new Dictionary<string, object> { {"id", hardware["id"]}, {"name", hardware["name"]}, {"sensors", sensors} });
        }
        return copy;
    }
    private static void Walk(object hardware, List<object> output, Stopwatch clock, int depth, ref int sensorCount)
    {
        if (depth > 16 || output.Count >= 64) throw new InvalidDataException("Hardware tree limit exceeded.");
        long acquired = (long)(Stopwatch.GetTimestamp() * 1000.0 / Stopwatch.Frequency);
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
            var entry = new Dictionary<string, object> {
                {"id", Get(sensor, "Identifier").ToString()}, {"name", Get(sensor, "Name").ToString()},
                {"kind", kind}, {"value", value}, {"age_ms", clock.ElapsedMilliseconds},
                // Boot-local monotonic acquisition identity, preserved by Aged
                // and BrokerCache; a cache read never makes a temperature newer.
                {"acquired_ms", acquired},
                {"control_available", kind == "control" && Get(sensor, "Control") != null}
            };
#if NATIVE_CONTROL_WORKER
            if (kind == "control" && Get(sensor, "Control") != null) entry.Add("support", Support(hardware, sensor));
#endif
            sensors.Add(entry);
        }
        output.Add(new Dictionary<string, object> { {"id", Get(hardware, "Identifier").ToString()}, {"name", Get(hardware, "Name").ToString()}, {"sensors", sensors} });
        foreach (object child in (IEnumerable)Get(hardware, "SubHardware")) Walk(child, output, clock, depth + 1, ref sensorCount);
    }
#if NATIVE_CONTROL_WORKER
    // Channels a crashed lower-tier worker left at 100% (until reboot); kept on
    // disk so a restarted snapshot worker still reports them.
    private static HashSet<string> held;
    private static string HeldPath { get { return Path.Combine(Path.GetDirectoryName(BrokerTransport.DirectoryPath), "mainboard-control-held.json"); } }
    // Inventory only (no I/O): which tier the chip can reach and whether LHM's
    // cached mode already reports software control (foreign takeover refused).
    private static Dictionary<string, object> Support(object hardware, object sensor)
    {
        object control = Get(sensor, "Control");
        string chip = NativeNctEvidence.AuditedChip(control);
        string id = Get(hardware, "Identifier").ToString() + "|" + Get(sensor, "Identifier").ToString();
        if (held == null) held = direct ? NativeStateFile.Load(HeldPath, true) : new HashSet<string>();
        return new Dictionary<string, object> {
            {"tier", chip != null ? "verified" : "lhm_supported"}, {"register_readback", chip != null},
            {"software_mode", Get(control, "ControlMode").ToString() == "Software"},
            {"held_full", held.Contains(id)}
        };
    }
#endif
    // Public SMBIOS data only; binds unrestricted unlocks to board and firmware.
    private static object Board()
    {
        try
        {
            object smbios = Get(computer, "SMBios");
            object board = smbios == null ? null : Get(smbios, "Board");
            object bios = smbios == null ? null : Get(smbios, "Bios");
            Func<object, string, string> text = delegate(object owner, string name) {
                object raw = owner == null ? null : Get(owner, name);
                string value = raw == null ? "" : raw.ToString().Trim();
                foreach (char c in value) if (Char.IsControl(c)) return "";
                return value.Length > 128 ? value.Substring(0, 128) : value;
            };
            return new Dictionary<string, object> {
                {"manufacturer", text(board, "ManufacturerName")}, {"product", text(board, "ProductName")},
                {"bios_version", text(bios, "Version")}
            };
        }
        catch (Exception error) { if (error is OutOfMemoryException) throw; return null; }
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
#if NATIVE_CONTROL_WORKER
        using (var journal=NativeControlJournal.CreateProtected())
        {
            // Every pending channel is checked on its own; a still-running original
            // worker of this service lifetime keeps its reservation.
            Func<Dictionary<string,object>,bool> active=delegate(Dictionary<string,object> current) {
                var provenance=current.ContainsKey("recovery") ? current["recovery"] as Dictionary<string,object> : null;
                return provenance != null && provenance.ContainsKey("worker_identity") &&
                    !Object.Equals(provenance["worker_identity"],BrokerTransport.CurrentProcessIdentity()) &&
                    provenance.ContainsKey("service_identity") && Object.Equals(provenance["service_identity"],BrokerTransport.ServiceIdentity()) &&
                    BrokerTransport.ProcessIdentityAlive((string)provenance["worker_identity"]);
            };
            journal.Recover(delegate(Dictionary<string,object> pending) {
                var matches=new List<object>();
                foreach (object board in (IEnumerable)Get(computer,"Hardware")) FindControl(board,(string)pending["sensor_id"],matches,0);
                if (matches.Count != 1 || Get(Get(matches[0],"Hardware"),"Identifier").ToString() != (string)pending["hardware_id"])
                    throw new InvalidOperationException("Pending native channel cannot be uniquely identified.");
                var recorded=pending.ContainsKey("recovery") ? pending["recovery"] as Dictionary<string,object> : null;
                if (recorded == null || !Object.Equals(recorded["kind"],"lhm_fallback_v1"))
                    return NativeNctEvidence.RecoverProtected(Get(matches[0],"Control"),recorded);
                bool heldFull;
                bool recovered=NativeControl.RecoverFallbackProtected(Get(matches[0],"Control"),recorded,out heldFull);
                if (heldFull)
                {
                    if (held == null) held = NativeStateFile.Load(HeldPath, true);
                    held.Add((string)pending["hardware_id"] + "|" + (string)pending["sensor_id"]);
                    try { NativeStateFile.Save(HeldPath, held); } catch (IOException) { } catch (UnauthorizedAccessException) { }
                }
                return recovered;
            },active);
        }
#endif
        var clock = Stopwatch.StartNew();
        int count = 0;
        // Motherboard (SuperIO fans/controls) first and on every snapshot; CPU/GPU
        // on their own measured cadence with their true age (UpdateCadence.cs).
        var items = new List<object>();
        foreach (object item in (IEnumerable)Get(computer, "Hardware"))
            if (IsMotherboard(item)) items.Insert(0, item); else items.Add(item);
        foreach (object item in items)
        {
            string key = Get(item, "Identifier").ToString();
            long now = Uptime.ElapsedMilliseconds;
            List<object> cached;
            if (cadence.Due(key, IsMotherboard(item), now) || !reused.TryGetValue(key, out cached))
            {
                var part = new List<object>();
                Walk(item, part, clock, 0, ref count);
                cadence.Record(key, now, Uptime.ElapsedMilliseconds - now);
                reused[key] = part;
                hardware.AddRange(part);
            }
            else hardware.AddRange(Aged(cached, cadence.AgeMs(key, now) + clock.ElapsedMilliseconds, ref count));
        }
        if (count == 0) return Empty("no_supported_hardware", "No supported motherboard sensor channels were reported; this does not prove no fans are connected.");
        string message = "Read-only motherboard channels. Fan/header pairing, PWM/DC mode and physical labels are unconfirmed; RGB uses a separate provider.";
        var slow = cadence.Slow();
        if (slow.Count > 0)
        {
            var parts = new List<string>();
            foreach (var pair in slow) if (parts.Count < 4) parts.Add(pair.Key + " " + pair.Value + " ms");
            message += " Slow sensor updates (sampled less often): " + String.Join(", ", parts.ToArray()) + ".";
        }
        var ready = Result("ready", message, hardware);
        object identity = Board();
        if (identity != null) ready.Add("board", identity);
        return ready;
#endif
    }
    private static object ProbeHundred(string sensorId)
    {
#if NATIVE_CONTROL_WORKER
        BrokerService.CheckInstallation(true);
        if (!direct || !System.Text.RegularExpressions.Regex.IsMatch(sensorId,"^/lpc/[a-z0-9]+/[0-9]+/control/[0-6]$"))
            throw new InvalidOperationException("Invalid protected 100-percent probe target.");
        if (computer == null) { var failure = Initialize(); if (failure != null) return failure; }
        var matches = new List<object>();
        foreach (object board in (IEnumerable)Get(computer,"Hardware")) FindControl(board,sensorId,matches,0);
        if (matches.Count != 1) throw new InvalidOperationException("Expected exactly one native control identity.");
        object control = Get(matches[0],"Control");
        var native = NativeControl.CreateProtected(control,true); // Hard floor100, never reduce.
        object hardware = Get(matches[0],"Hardware");
        object before = ProbeReadings(hardware);
        using (var journal = NativeControlJournal.CreateProtected())
        {
            var owned = JournaledNativeControl.CreateProtected(native,journal);
            object applied;
            try
            {
                owned.Acquire();
                System.Threading.Thread.Sleep(3000);
                applied = ProbeReadings(hardware);
            }
            finally { owned.Release(); }
            System.Threading.Thread.Sleep(250);
            return new { version=1, status="completed",control_id=sensorId,requested_percent=100,
                register_write_verified=true,firmware_return_verified=owned.FirmwareReturnVerified,
                journal_cleared=owned.JournalCleared,before=before,applied=applied,returned=ProbeReadings(hardware) };
        }
#else
        return Empty("faulted","Hardware probe is absent from this build.");
#endif
    }
#if NATIVE_CONTROL_WORKER
    private static void FindSensor(object hardware, string id, string type, List<object> matches, int depth)
    {
        if (depth > 16) throw new InvalidDataException("Hardware tree depth exceeded.");
        foreach (object sensor in (IEnumerable)Get(hardware,"Sensors"))
            if (Get(sensor,"SensorType").ToString() == type && Get(sensor,"Identifier").ToString() == id) matches.Add(sensor);
        foreach (object child in (IEnumerable)Get(hardware,"SubHardware")) FindSensor(child,id,type,matches,depth+1);
    }
    private static void FindControl(object hardware, string id, List<object> matches, int depth)
    {
        if (depth > 16) throw new InvalidDataException("Hardware tree depth exceeded.");
        foreach (object sensor in (IEnumerable)Get(hardware,"Sensors"))
            if (Get(sensor,"SensorType").ToString() == "Control" && Get(sensor,"Identifier").ToString() == id) matches.Add(sensor);
        foreach (object child in (IEnumerable)Get(hardware,"SubHardware")) FindControl(child,id,matches,depth+1);
    }
    private static object ProbeReadings(object hardware)
    {
        Invoke(hardware,"Update");
        var readings = new List<object>();
        foreach (object sensor in (IEnumerable)Get(hardware,"Sensors"))
        {
            string kind = Get(sensor,"SensorType").ToString();
            if (kind != "Fan" && kind != "Control") continue;
            readings.Add(new { id=Get(sensor,"Identifier").ToString(),kind=kind,value=Get(sensor,"Value") });
        }
        return readings;
    }
#endif
    private static int Main(string[] args)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        if (args.Length == 6 && args[0] == "--request-control-authorization") return NativeControlBroker.RequestAuthorization(args);
        if ((args.Length == 1 || args.Length == 2) && args[0] == "--control-relay") return NativeControlBroker.Relay(args.Length == 2 ? args[1] : null);
        if (args.Length == 2 && args[0] == "--request-background-config") return NativeControlBroker.RequestBackgroundConfiguration(args);
        if (args.Length == 2 && args[0] == "--background") return NativeControlBroker.Background(args[1]);
#if NATIVE_CONTROL_WORKER
        if (args.Length == 4 && args[0] == "--control-protected-session")
        {
            // Only the registered protected service launches this dedicated worker.
            // No fan is acquired until the service-only pipe peer authenticates.
            BrokerService.CheckInstallation(true);
            library=BrokerService.RuntimePath; direct=true;
            if (!System.Text.RegularExpressions.Regex.IsMatch(args[2],"^/lpc/[a-z0-9]+/[0-9]+/control/[0-6]$")) return 2;
            object unavailable=Initialize(true);
            if (unavailable != null) throw new InvalidOperationException("Native runtime unavailable.");
            var controls=new List<object>();
            foreach (object hardware in (IEnumerable)Get(computer,"Hardware")) FindControl(hardware,args[2],controls,0);
            if (controls.Count != 1) throw new InvalidOperationException("Control identity is not unique.");
            Func<string,object> findTemperature = delegate(string id) {
                var found=new List<object>();
                foreach (object hardware in (IEnumerable)Get(computer,"Hardware")) FindSensor(hardware,id,"Temperature",found,0);
                return found.Count == 1 ? found[0] : null;
            };
            return NativeControlWorker.Run(Get(controls[0],"Control"),args[1],args[3],findTemperature);
        }
#endif
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
                bool probe = direct && request.StartsWith("probe100 ",StringComparison.Ordinal);
                if (request != "snapshot" && !probe) return 2;
                string json;
                try
                {
                    string broker = null;
                    if (!direct)
                    {
                        try { broker = BrokerTransport.Snapshot(); }
                        catch (Win32Exception error) { if (error.NativeErrorCode != 2) throw; }
                    }
                    json = broker ?? Json.Serialize(probe ? ProbeHundred(request.Substring(9)) : Snapshot());
                }
                catch (Exception error)
                {
                    Exception cause = error is TargetInvocationException && error.InnerException != null ? error.InnerException : error;
                    string detail = cause is InvalidDataException || probe ? " " + cause.Message.Substring(0,Math.Min(cause.Message.Length,256)) : "";
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
