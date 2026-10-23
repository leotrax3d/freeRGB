// Protected background fan control: saved per-channel curves run in the SYSTEM
// service from boot, without a GUI or daemon. Each channel is driven through its
// own protected worker (same binding, journal, watchdog and fail-safe return as
// live sessions). The curve/plausibility core below is pure and unit-tested; the
// process supervision part exists only in the installed service build.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

// Mirrors freergb_control::FanCurve::duty + CurveController (shared vectors in
// crates/freergb-control/tests/fixtures/curve_vectors.json).
internal sealed class BackgroundCurve
{
    private readonly double[] temperatures, duties;
    private readonly double minimum, critical, hysteresis;
    private double? lastTemperature;
    private double lastDuty;
    private double? rise, fall;     // Optional step limits, percent per second.
    private long outputAt = -1;
    private double output;
    internal BackgroundCurve(double[] temperatures, double[] duties, double minimum, double critical, double hysteresis, double? rise = null, double? fall = null)
    {
        foreach (double? rate in new[] { rise, fall })
            if (rate.HasValue && (!Finite(rate.Value) || rate.Value < 1 || rate.Value > 100)) throw new InvalidDataException("Step limits must be 1..100 percent per second.");
        this.rise = rise; this.fall = fall;
        if (temperatures == null || duties == null || temperatures.Length != duties.Length || temperatures.Length < 2 || temperatures.Length > 16)
            throw new InvalidDataException("A curve needs 2..16 points.");
        if (!Finite(minimum) || minimum < 0 || minimum > 100 || !Finite(critical) || critical < 40 || critical > 110 || !Finite(hysteresis) || hysteresis < 0 || hysteresis > 10)
            throw new InvalidDataException("Invalid curve limits.");
        for (int i = 0; i < temperatures.Length; i++)
        {
            if (!Finite(temperatures[i]) || !Finite(duties[i]) || temperatures[i] < -20 || temperatures[i] > 110 || duties[i] < minimum || duties[i] > 100)
                throw new InvalidDataException("Invalid curve point.");
            if (i > 0 && (temperatures[i] <= temperatures[i-1] || duties[i] < duties[i-1]))
                throw new InvalidDataException("Temperatures must increase; duty must not decrease.");
        }
        if (temperatures[temperatures.Length-1] > critical) throw new InvalidDataException("Curve extends beyond critical temperature.");
        this.temperatures = temperatures; this.duties = duties; this.minimum = minimum; this.critical = critical; this.hysteresis = hysteresis;
        lastDuty = 0;
    }
    private static bool Finite(double value) { return !Double.IsNaN(value) && !Double.IsInfinity(value); }
    internal double Minimum { get { return minimum; } }
    internal double Critical { get { return critical; } }
    internal double Target(double? temperature, long ageMs)
    {
        if (ageMs > 3000 || !temperature.HasValue || !Finite(temperature.Value) || temperature.Value < -20 || temperature.Value > 150) return 100;
        double t = temperature.Value;
        if (t >= critical) return 100;
        if (t <= temperatures[0]) return duties[0];
        for (int i = 1; i < temperatures.Length; i++)
            if (t <= temperatures[i])
                return duties[i-1] + (t - temperatures[i-1]) / (temperatures[i] - temperatures[i-1]) * (duties[i] - duties[i-1]);
        return duties[duties.Length-1];
    }
    // React to warming immediately; suppress small downward oscillations.
    internal double Update(double? temperature, long ageMs)
    {
        double target = Target(temperature, ageMs);
        bool valid = temperature.HasValue && Finite(temperature.Value) && temperature.Value >= -20 && temperature.Value <= 150 && ageMs <= 3000;
        if (!valid) { lastTemperature = null; lastDuty = 100; return 100; }
        double t = temperature.Value;
        if (target >= lastDuty || !lastTemperature.HasValue || t <= lastTemperature.Value - hysteresis)
        { lastDuty = target; lastTemperature = t; }
        return lastDuty;
    }
    // Update plus step limits; invalid/stale sensors and critical heat jump to 100%.
    internal double UpdateAt(double? temperature, long ageMs, long now)
    {
        double target = Update(temperature, ageMs);
        bool emergency = target >= 100 && (!lastTemperature.HasValue || (temperature.HasValue && temperature.Value >= critical));
        double duty = target;
        if (outputAt >= 0 && !emergency)
        {
            double seconds = Math.Max(0, now - outputAt) / 1000.0;
            if (rise.HasValue) duty = Math.Min(duty, output + rise.Value * seconds);
            if (fall.HasValue) duty = Math.Max(duty, output - fall.Value * seconds);
        }
        outputAt = now; output = duty;
        return duty;
    }
}

// Expected RPM per duty from the guided channel test. A deviation that persists
// after the fan settled is a fault (LHM-supported) or only a warning (unrestricted).
internal sealed class RpmPlausibility
{
    private readonly int[] duties;
    private readonly double[] rpms;
    private const long SettleMs = 4000, PersistMs = 5000;
    private long changedAt = -1, deviatingSince = -1;
    private int lastDuty = -1;
    internal RpmPlausibility(int[] duties, double[] rpms)
    {
        if (duties == null || rpms == null || duties.Length != rpms.Length || duties.Length < 2 || duties.Length > 8)
            throw new InvalidDataException("Plausibility needs 2..8 tested duties.");
        for (int i = 0; i < duties.Length; i++)
        {
            if (duties[i] < 40 || duties[i] > 100 || Double.IsNaN(rpms[i]) || rpms[i] < 100 || rpms[i] > 100000)
                throw new InvalidDataException("Invalid tested duty/RPM.");
            if (i > 0 && (duties[i] <= duties[i-1] || rpms[i] < rpms[i-1])) throw new InvalidDataException("Tested RPM must rise with duty.");
        }
        this.duties = duties; this.rpms = rpms;
    }
    // Expected RPM for a duty inside the tested range; null below/above it.
    internal double? Expected(int duty)
    {
        if (duty < duties[0] || duty > duties[duties.Length-1]) return null;
        for (int i = 1; i < duties.Length; i++)
            if (duty <= duties[i])
                return rpms[i-1] + (double)(duty - duties[i-1]) / (duties[i] - duties[i-1]) * (rpms[i] - rpms[i-1]);
        return rpms[rpms.Length-1];
    }
    internal static bool Within(double expected, double rpm) { return rpm >= expected * 0.6 - 150 && rpm <= expected * 1.4 + 150; }
    // Returns "rpm_implausible" once a deviation persisted; null otherwise.
    internal string Observe(int duty, double rpm, long now)
    {
        if (duty != lastDuty) { if (lastDuty < 0 || Math.Abs(duty - lastDuty) >= 5) changedAt = now; lastDuty = duty; }
        double? expected = Expected(duty);
        bool deviating = expected.HasValue && now - changedAt >= SettleMs && !Within(expected.Value, rpm);
        if (!deviating) { deviatingSince = -1; return null; }
        if (deviatingSince < 0) deviatingSince = now;
        return now - deviatingSince >= PersistMs ? "rpm_implausible" : null;
    }
}

internal sealed class BackgroundChannelConfig
{
    internal string ControlId, TemperatureId, RpmId, Role;
    internal NativeTier Tier;
    internal bool PumpUnlocked;
    internal int Floor, Critical;
    internal BackgroundCurve Curve;
    internal RpmPlausibility Plausibility;
    internal Dictionary<string,object> Binding()
    {
        var binding = new Dictionary<string,object> { {"version",1},{"temperature_id",TemperatureId},{"rpm_id",RpmId},{"floor_percent",Floor},{"critical_c",Critical},{"role",Role} };
        if (Tier != NativeTier.Verified) binding.Add("tier",NativeTiers.Name(Tier));
        if (PumpUnlocked) binding.Add("pump_unlocked",true);
        return binding;
    }
    private static readonly System.Text.RegularExpressions.Regex Source = new System.Text.RegularExpressions.Regex("^/lpc/[a-z0-9]+/[0-9]+/(control|temperature|fan)/[0-9]+$");
    private static string Base(string id, string kind)
    {
        if (id == null || id.Length > 128 || !Source.IsMatch(id)) throw new InvalidDataException("Invalid native source identity.");
        int marker = id.LastIndexOf("/" + kind + "/", StringComparison.Ordinal);
        if (marker < 0) throw new InvalidDataException("Native source has the wrong kind.");
        return id.Substring(0, marker);
    }
    private static double Number(object raw)
    {
        if (raw is int) return (int)raw;
        if (raw is decimal) return (double)(decimal)raw;
        if (raw is double) return (double)raw;
        throw new InvalidDataException("Expected a number.");
    }
    internal const int MaxChannels = 8;
    // Version 1: {"version":1,"channels":[...]} written only by the elevated,
    // consented configuration command. Everything is re-validated here.
    internal static List<BackgroundChannelConfig> ParseAll(string text)
    {
        if (text == null || Encoding.UTF8.GetByteCount(text) > 32768) throw new InvalidDataException("Background configuration too large.");
        var document = new JavaScriptSerializer { MaxJsonLength = 32768, RecursionLimit = 10 }.DeserializeObject(text) as Dictionary<string,object>;
        if (document == null || document.Count != 2 || !Object.Equals(document["version"],1) || !(document.ContainsKey("channels") && document["channels"] is object[]))
            throw new InvalidDataException("Unsupported background configuration.");
        var list = (object[])document["channels"];
        if (list.Length > MaxChannels) throw new InvalidDataException("Too many background channels.");
        var result = new List<BackgroundChannelConfig>();
        var seen = new HashSet<string>();
        foreach (object raw in list)
        {
            var value = raw as Dictionary<string,object>;
            var allowed = new HashSet<string> { "control_id","temperature_id","rpm_id","role","tier","floor_percent","critical_c","pump_unlocked","curve","plausibility" };
            if (value == null) throw new InvalidDataException("Invalid background channel.");
            foreach (string key in value.Keys) if (!allowed.Contains(key)) throw new InvalidDataException("Unknown background channel field: " + key);
            var config = new BackgroundChannelConfig {
                ControlId = value.ContainsKey("control_id") ? value["control_id"] as string : null,
                TemperatureId = value.ContainsKey("temperature_id") ? value["temperature_id"] as string : null,
                RpmId = value.ContainsKey("rpm_id") ? value["rpm_id"] as string : null,
            };
            string hardware = Base(config.ControlId,"control");
            if (Base(config.RpmId,"fan") != hardware)
                throw new InvalidDataException("The RPM sensor must be on the control's hardware.");
            NativeSources.Temperatures(config.TemperatureId); // SuperIO, CPU or GPU; up to three, hottest wins.
            if (!seen.Add(config.ControlId)) throw new InvalidDataException("Duplicate background channel.");
            if (!(value.ContainsKey("floor_percent") && value["floor_percent"] is int) || !(value.ContainsKey("critical_c") && value["critical_c"] is int))
                throw new InvalidDataException("Background floor/critical must be integers.");
            config.Floor = (int)value["floor_percent"]; config.Critical = (int)value["critical_c"];
            if (config.Critical < 40 || config.Critical > 100) throw new InvalidDataException("Critical temperature must be 40..100 C.");
            var rules = new Dictionary<string,object> { {"role", value.ContainsKey("role") ? value["role"] : null} };
            if (value.ContainsKey("tier") && !Object.Equals(value["tier"],"verified")) rules.Add("tier",value["tier"]);
            if (value.ContainsKey("pump_unlocked")) rules.Add("pump_unlocked",value["pump_unlocked"]);
            bool pump;
            NativeAuthorizationRules.Validate(rules,config.Floor,out config.Tier,out pump,out config.PumpUnlocked);
            config.Role = pump ? "pump" : "fan";
            if (!Object.Equals(rules["role"],config.Role)) throw new InvalidDataException("Background channel role required.");
            var curve = value.ContainsKey("curve") ? value["curve"] as Dictionary<string,object> : null;
            int curveKeys = 4 + (curve != null && curve.ContainsKey("max_rise_percent_per_s") ? 1 : 0) + (curve != null && curve.ContainsKey("max_fall_percent_per_s") ? 1 : 0);
            if (curve == null || curve.Count != curveKeys || !(curve.ContainsKey("points") && curve["points"] is object[])) throw new InvalidDataException("Background curve required.");
            var points = (object[])curve["points"];
            var temperatures = new double[points.Length]; var duties = new double[points.Length];
            for (int i = 0; i < points.Length; i++)
            {
                var point = points[i] as Dictionary<string,object>;
                if (point == null || point.Count != 2 || !point.ContainsKey("temperature_c") || !point.ContainsKey("duty_percent")) throw new InvalidDataException("Invalid curve point.");
                temperatures[i] = Number(point["temperature_c"]); duties[i] = Number(point["duty_percent"]);
            }
            config.Curve = new BackgroundCurve(temperatures,duties,Number(curve["minimum_percent"]),Number(curve["critical_c"]),Number(curve["hysteresis_c"]),
                curve.ContainsKey("max_rise_percent_per_s") ? Number(curve["max_rise_percent_per_s"]) : (double?)null,
                curve.ContainsKey("max_fall_percent_per_s") ? Number(curve["max_fall_percent_per_s"]) : (double?)null);
            if (config.Curve.Minimum < config.Floor || config.Curve.Critical > config.Critical)
                throw new InvalidDataException("Curve must respect the channel floor and critical temperature.");
            if (value.ContainsKey("plausibility") && value["plausibility"] != null)
            {
                var bands = value["plausibility"] as object[];
                if (bands == null) throw new InvalidDataException("Invalid plausibility data.");
                var tested = new int[bands.Length]; var rpm = new double[bands.Length];
                for (int i = 0; i < bands.Length; i++)
                {
                    var band = bands[i] as Dictionary<string,object>;
                    if (band == null || band.Count != 2 || !(band.ContainsKey("duty_percent") && band["duty_percent"] is int) || !band.ContainsKey("rpm"))
                        throw new InvalidDataException("Invalid plausibility band.");
                    tested[i] = (int)band["duty_percent"]; rpm[i] = Number(band["rpm"]);
                }
                config.Plausibility = new RpmPlausibility(tested,rpm);
            }
            // LHM-supported reduction is only allowed with evidence of a passed test.
            if (config.Tier == NativeTier.LhmSupported && config.Floor < 100 && config.Plausibility == null)
                throw new InvalidDataException("LHM-supported reduced duty requires a passed channel test.");
            result.Add(config);
        }
        return result;
    }
}

// Receipt fields the controller needs; implemented by the protected session client.
internal interface IBackgroundSession
{
    string State { get; }
    string Reason { get; }
    string Warning { get; }
    double? Temperature { get; }
    double? Rpm { get; }
    long AgeMs { get; }
    void SetDuty(int duty);
    void Close();
}

// One control step per tick. Never commands below the protected floor; any
// non-active receipt ends the run (the worker already returned/held the channel).
internal sealed class BackgroundChannelController
{
    private readonly BackgroundChannelConfig config;
    internal string State = "starting", Fault, Warning;
    internal int Duty = 100;
    internal BackgroundChannelController(BackgroundChannelConfig config) { this.config = config; }
    internal bool Step(IBackgroundSession session, long now)
    {
        if (session.State != "active") { State = "faulted"; Fault = "worker:" + (session.Reason ?? session.State); return false; }
        Warning = session.Warning;
        if (config.Plausibility != null && session.Rpm.HasValue)
        {
            string deviation = config.Plausibility.Observe(Duty, session.Rpm.Value, now);
            if (deviation != null)
            {
                if (config.Tier != NativeTier.Unrestricted)
                {
                    State = "faulted"; Fault = deviation;
                    session.Close(); // 100%, then return to the captured firmware mode.
                    return false;
                }
                Warning = deviation;
            }
        }
        double target = config.Curve.UpdateAt(session.Temperature, session.AgeMs, now);
        int duty = (int)Math.Ceiling(target);
        if (duty < config.Floor) duty = config.Floor;
        if (duty > 100) duty = 100;
        session.SetDuty(duty);
        Duty = duty;
        State = "running";
        return true;
    }
}

#if BROKER_INSTALLER && !NATIVE_CONTROL_TESTS
// Service-side supervision. Workers are separate processes; each reserves its own
// journal entry. Channels are suspended while an interactive session uses them.
internal sealed class BackgroundCooling : IDisposable
{
    internal const string ConfigName = "mainboard-control-channels.json", StatusName = "mainboard-control-status.json";
    private sealed class Channel
    {
        internal BackgroundChannelConfig Config;
        internal BackgroundChannelController Controller;
        internal Thread Thread;
        internal volatile bool Suspended;
        internal int Restarts;
        internal long RetryAt;
    }
    private readonly object gate = new object();
    private readonly ManualResetEvent stop = new ManualResetEvent(false);
    private readonly List<Channel> channels = new List<Channel>();
    private Thread statusThread;
    private static string Root { get { return Path.GetDirectoryName(BrokerTransport.DirectoryPath); } }
    private static string PausePath { get { return Path.Combine(Root, "mainboard-control-paused.json"); } }
    internal void Start()
    {
        BrokerService.CheckInstallation(false);
        // A user pause lasts until resumed, also across service restarts/reboots.
        paused = NativeStateFile.Load(PausePath, false).Contains("background");
        string path = Path.Combine(Root, ConfigName);
        List<BackgroundChannelConfig> configs = new List<BackgroundChannelConfig>();
        if (File.Exists(path))
        {
            BrokerService.CheckProtectedPath(path);
            try { configs = BackgroundChannelConfig.ParseAll(File.ReadAllText(path)); }
            catch (Exception error) { if (error is OutOfMemoryException) throw; WriteStatus("invalid_configuration"); return; }
        }
        lock (gate)
            foreach (var config in configs)
            {
                var channel = new Channel { Config = config, Controller = new BackgroundChannelController(config), Suspended = paused };
                channel.Thread = new Thread(delegate() { Run(channel); }) { IsBackground = true, Name = "freeRGB background cooling" };
                channels.Add(channel);
                channel.Thread.Start();
            }
        statusThread = new Thread(delegate() { while (!stop.WaitOne(2000)) WriteStatus(null); }) { IsBackground = true, Name = "freeRGB cooling status" };
        statusThread.Start();
    }
    // Interactive sessions take precedence: stop and return the channel first.
    internal void Suspend(string controlId)
    {
        Channel target = null;
        lock (gate) foreach (var c in channels) if (c.Config.ControlId == controlId) target = c;
        if (target == null) return;
        target.Suspended = true;
        var deadline = Stopwatch.StartNew();
        while (target.Controller.State == "running" && deadline.ElapsedMilliseconds < 5000) Thread.Sleep(50);
        if (target.Controller.State == "running") throw new InvalidOperationException("Background channel did not release in time.");
    }
    // User pause (tray): every channel returns to firmware until resumed.
    private volatile bool paused;
    internal bool Paused { get { return paused; } }
    internal void PauseAll(bool pause)
    {
        paused = pause;
        try { NativeStateFile.Save(PausePath, pause ? new[] { "background" } : new string[0]); }
        catch (IOException) { } catch (UnauthorizedAccessException) { }
        lock (gate) foreach (var c in channels) c.Suspended = pause;
        WriteStatus(null);
    }
    internal void Resume(string controlId)
    {
        if (paused) return; // A user pause outlasts an interactive session.
        lock (gate) foreach (var c in channels) if (c.Config.ControlId == controlId) c.Suspended = false;
    }
    private void Run(Channel channel)
    {
        var clock = Stopwatch.StartNew();
        while (!stop.WaitOne(0))
        {
            if (channel.Suspended || clock.ElapsedMilliseconds < channel.RetryAt || channel.Controller.Fault == "rpm_implausible")
            {
                if (channel.Suspended) channel.Controller.State = "suspended";
                stop.WaitOne(500);
                continue;
            }
            try { Drive(channel, clock); }
            catch (Exception error) { if (error is OutOfMemoryException) throw; channel.Controller.State = "faulted"; channel.Controller.Fault = error.GetType().Name; }
            // Bounded automatic restart after worker faults; plausibility faults
            // stay latched until the user re-tests and saves the channel again.
            if (channel.Controller.Fault != "rpm_implausible")
            {
                channel.Restarts++;
                channel.RetryAt = clock.ElapsedMilliseconds + (channel.Restarts > 3 ? 600000 : 30000);
            }
        }
    }
    private void Drive(Channel channel, Stopwatch clock)
    {
        string binding = Convert.ToBase64String(Encoding.UTF8.GetBytes(new JavaScriptSerializer().Serialize(channel.Config.Binding())));
        string name = "freergb-control-worker-" + Guid.NewGuid().ToString("N");
        var start = new ProcessStartInfo(Path.Combine(BrokerTransport.DirectoryPath,"freergb-mainboard.exe"),
            "--control-protected-session " + name + " " + channel.Config.ControlId + " " + binding) {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true,
            WorkingDirectory = Environment.GetFolderPath(Environment.SpecialFolder.System)
        };
        using (var worker = new Process { StartInfo = start })
        {
            worker.Start();
            try
            {
                if (BrokerTransport.ReadRecord(worker.StandardOutput.BaseStream,10000,64).TrimEnd('\r') != "ready") throw new InvalidDataException("Background worker not ready.");
                using (var pipe = BrokerTransport.Connect(name))
                using (var session = NativeSessionClient.CreateProtected(pipe,(uint)worker.Id))
                {
                    channel.Controller.Fault = null;
                    channel.Restarts = 0;
                    var view = new SessionView(session);
                    while (!stop.WaitOne(100) && !channel.Suspended)
                        if (!channel.Controller.Step(view, clock.ElapsedMilliseconds)) return;
                    // Stop/suspend: explicit close (100%, then firmware return).
                    if (view.State == "active") session.Close();
                    channel.Controller.State = channel.Suspended ? "suspended" : "stopped";
                }
            }
            finally { if (!worker.WaitForExit(2000)) { worker.Kill(); worker.WaitForExit(2000); } }
        }
    }
    private sealed class SessionView : IBackgroundSession
    {
        private readonly NativeSessionClient session;
        internal SessionView(NativeSessionClient session) { this.session = session; }
        public string State { get { return session.Last.State; } }
        public string Reason { get { return session.Last.Reason; } }
        public string Warning { get { return session.Last.Warning; } }
        public double? Temperature { get { return session.Last.Temperature; } }
        public double? Rpm { get { return session.Last.Rpm; } }
        public long AgeMs { get { return session.Last.AgeMs; } }
        public void SetDuty(int duty) { session.SetDuty(duty); }
        public void Close() { session.Close(); }
    }
    // Readable by the user-level daemon (Program Files ACL); no secrets inside.
    private void WriteStatus(string error)
    {
        try
        {
            var entries = new List<object>();
            lock (gate)
                foreach (var c in channels)
                    entries.Add(new { control_id = c.Config.ControlId, tier = NativeTiers.Name(c.Config.Tier), state = c.Controller.State,
                        duty_percent = c.Controller.Duty, fault = c.Controller.Fault, warning = c.Controller.Warning });
            string path = Path.Combine(Root, StatusName), temporary = path + ".tmp";
            File.WriteAllText(temporary, new JavaScriptSerializer().Serialize(new { version = 1, error = error, paused = paused, updated_utc = DateTime.UtcNow.ToString("o"), channels = entries }));
            if (File.Exists(path)) File.Replace(temporary, path, null); else File.Move(temporary, path);
        }
        catch (IOException) { } catch (UnauthorizedAccessException) { }
    }
    public void Dispose()
    {
        stop.Set();
        List<Channel> copy;
        lock (gate) copy = new List<Channel>(channels);
        foreach (var c in copy) if (!c.Thread.Join(8000)) throw new InvalidOperationException("Background channel did not stop; preserve recovery state.");
        if (statusThread != null) statusThread.Join(3000);
        WriteStatus("stopped");
    }
}
#endif
