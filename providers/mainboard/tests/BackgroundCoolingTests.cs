// Background controller core: curve parity with Rust, RPM plausibility, strict
// configuration validation and controller steps. Fake sessions; no hardware.
using System;
using System.Collections.Generic;
using System.IO;
using System.Web.Script.Serialization;

internal sealed class FakeBackgroundSession : IBackgroundSession
{
    public string State { get; set; }
    public string Reason { get; set; }
    public string Warning { get; set; }
    public double? Temperature { get; set; }
    public double? Rpm { get; set; }
    public long AgeMs { get; set; }
    internal readonly List<int> Duties = new List<int>();
    internal bool Closed;
    public void SetDuty(int duty) { Duties.Add(duty); }
    public void Close() { Closed = true; }
}

internal static class BackgroundCoolingTests
{
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action, string message)
    {
        try { action(); } catch (InvalidDataException) { return; } catch (ArgumentException) { return; } catch (InvalidOperationException) { return; }
        throw new Exception(message);
    }
    internal static void Run(string repository)
    {
        Vectors(Path.Combine(repository,"crates","freergb-control","tests","fixtures","curve_vectors.json"));
        Plausibility();
        Configuration();
        Controller();
        Console.WriteLine("PASS: background curve parity with Rust vectors, RPM plausibility, strict protected configuration and controller floors/faults. Synthetic only.");
    }
    private static double D(object raw) { return Convert.ToDouble(raw,System.Globalization.CultureInfo.InvariantCulture); }
    private static void Vectors(string path)
    {
        var document = (Dictionary<string,object>)new JavaScriptSerializer().DeserializeObject(File.ReadAllText(path));
        var curve = (Dictionary<string,object>)document["curve"];
        var points = (object[])curve["points"];
        var t = new double[points.Length]; var d = new double[points.Length];
        for (int i = 0; i < points.Length; i++) { var p = (Dictionary<string,object>)points[i]; t[i] = D(p["temperature_c"]); d[i] = D(p["duty_percent"]); }
        var evaluator = new BackgroundCurve(t,d,D(curve["minimum_percent"]),D(curve["critical_c"]),D(curve["hysteresis_c"]));
        foreach (Dictionary<string,object> step in (object[])document["steps"])
        {
            double? temperature = step["temperature_c"] == null ? (double?)null : D(step["temperature_c"]);
            double duty = evaluator.Update(temperature,Convert.ToInt64(step["age_ms"]));
            Assert(Math.Abs(duty - D(step["duty_percent"])) < 1e-3,"C# curve diverges from the shared Rust vectors at " + new JavaScriptSerializer().Serialize(step));
        }
    }
    private static void Plausibility()
    {
        var p = new RpmPlausibility(new[] { 50, 70, 100 }, new[] { 800.0, 1100.0, 1500.0 });
        Assert(p.Expected(60) == 950 && p.Expected(40) == null && p.Expected(100) == 1500,"Expected RPM interpolation changed.");
        Assert(p.Observe(70,1100,0) == null,"Plausible RPM faulted.");
        Assert(p.Observe(70,100,1000) == null,"Deviation during the settle period faulted.");
        Assert(p.Observe(70,100,4500) == null && p.Observe(70,100,9000) == null && p.Observe(70,100,9600) == "rpm_implausible","Persistent deviation was not detected after settling.");
        var q = new RpmPlausibility(new[] { 50, 100 }, new[] { 800.0, 1500.0 });
        q.Observe(100,1500,0);
        Assert(q.Observe(100,1490,10000) == null && q.Observe(30,0,20000) == null,"Untested duties or small deviations faulted.");
        Reject(delegate { new RpmPlausibility(new[] { 50 }, new[] { 800.0 }); },"Single tested duty accepted.");
        Reject(delegate { new RpmPlausibility(new[] { 70, 50 }, new[] { 800.0, 900.0 }); },"Unordered tested duties accepted.");
        Reject(delegate { new RpmPlausibility(new[] { 50, 70 }, new[] { 900.0, 800.0 }); },"Falling tested RPM accepted.");
        Reject(delegate { new RpmPlausibility(new[] { 30, 70 }, new[] { 500.0, 800.0 }); },"Tested duty below 40% accepted.");
    }
    private static string Channel(string extra, int floor, string tier, string role)
    {
        return "{\"control_id\":\"/lpc/it8689e/0/control/1\",\"temperature_id\":\"/lpc/it8689e/0/temperature/0\",\"rpm_id\":\"/lpc/it8689e/0/fan/1\"," +
            "\"role\":\"" + role + "\",\"tier\":\"" + tier + "\",\"floor_percent\":" + floor + ",\"critical_c\":85," +
            "\"curve\":{\"points\":[{\"temperature_c\":30,\"duty_percent\":" + floor + "},{\"temperature_c\":70,\"duty_percent\":100}],\"minimum_percent\":" + floor + ",\"critical_c\":85,\"hysteresis_c\":2}" + extra + "}";
    }
    private const string Bands = ",\"plausibility\":[{\"duty_percent\":50,\"rpm\":800},{\"duty_percent\":70,\"rpm\":1100},{\"duty_percent\":100,\"rpm\":1500}]";
    private static string Config(params string[] channels) { return "{\"version\":1,\"channels\":[" + String.Join(",",channels) + "]}"; }
    private static void Configuration()
    {
        var parsed = BackgroundChannelConfig.ParseAll(Config(Channel(Bands,40,"lhm_supported","fan")));
        Assert(parsed.Count == 1 && parsed[0].Tier == NativeTier.LhmSupported && parsed[0].Plausibility != null,"Tested LHM channel rejected.");
        var binding = parsed[0].Binding();
        Assert((string)binding["tier"] == "lhm_supported" && (int)binding["floor_percent"] == 40 && (string)binding["role"] == "fan" && !binding.ContainsKey("pump_unlocked"),"Worker binding does not carry the channel limits.");
        var verified = BackgroundChannelConfig.ParseAll(Config(Channel("",40,"verified","fan")))[0].Binding();
        Assert(!verified.ContainsKey("tier"),"Verified background channel did not use the legacy binding.");
        BackgroundChannelConfig.ParseAll(Config(Channel("",0,"unrestricted","fan")));
        BackgroundChannelConfig.ParseAll(Config(Channel(",\"pump_unlocked\":true",20,"unrestricted","pump")));
        BackgroundChannelConfig.ParseAll(Config(Channel("",100,"lhm_supported","pump")));
        BackgroundChannelConfig.ParseAll(Config());
        foreach (string bad in new[] {
            Config(Channel("",40,"lhm_supported","fan")),                 // untested LHM reduction
            Config(Channel(Bands,0,"lhm_supported","fan")),               // fan stop without unlock
            Config(Channel("",30,"verified","fan")),
            Config(Channel("",60,"unrestricted","pump")),                 // pump without second confirmation
            Config(Channel("",60,"lhm_supported","pump")),
            Config(Channel(",\"extra\":1",40,"verified","fan")),
            Config(Channel(Bands,40,"lhm_supported","fan"),Channel(Bands,40,"lhm_supported","fan")),
            Config(Channel("",40,"verified","fan").Replace("temperature/0","temperature/0x")),
            Config(Channel("",40,"verified","fan").Replace("/lpc/it8689e/0/fan/1","/lpc/nct6701d/0/fan/1")),
            Config(Channel("",40,"verified","fan").Replace("\"critical_c\":85,\"curve\"","\"critical_c\":120,\"curve\"")),
            Config(Channel("",40,"verified","fan").Replace("\"hysteresis_c\":2","\"hysteresis_c\":2,\"x\":1")),
            "{\"version\":2,\"channels\":[]}",
            "not json" })
            Reject(delegate { BackgroundChannelConfig.ParseAll(bad); },"Unsafe background configuration accepted: " + bad);
        var many = new List<string>();
        for (int i = 0; i < 9; i++) many.Add(Channel("",40,"verified","fan").Replace("control/1","control/" + i));
        Reject(delegate { BackgroundChannelConfig.ParseAll(Config(many.ToArray())); },"More than eight background channels accepted.");
    }
    private static void Controller()
    {
        var config = BackgroundChannelConfig.ParseAll(Config(Channel(Bands,40,"lhm_supported","fan")))[0];
        var controller = new BackgroundChannelController(config);
        var session = new FakeBackgroundSession { State = "active", Temperature = 20, Rpm = 900, AgeMs = 100 };
        Assert(controller.Step(session,0) && session.Duties[0] == 40 && controller.State == "running","Cool channel did not run at the tier floor.");
        session.Temperature = 90;
        Assert(controller.Step(session,500) && session.Duties[1] == 100,"Critical temperature did not command 100%.");
        session.AgeMs = 5000; session.Temperature = 20;
        Assert(controller.Step(session,600) && session.Duties[2] == 100,"Stale telemetry did not command 100%.");
        // Persistent implausible RPM at 40% (expected ~below tested range: no check) vs 70%.
        session.AgeMs = 100; session.Temperature = 50; session.Rpm = 50;
        long now = 1000;
        bool running = true;
        while (running && now < 30000) { running = controller.Step(session,now); now += 500; }
        Assert(!running && controller.Fault == "rpm_implausible" && session.Closed,"LHM-supported plausibility fault did not close (return) the channel.");

        var unlocked = BackgroundChannelConfig.ParseAll(Config(Channel(Bands,0,"unrestricted","fan")))[0];
        var free = new BackgroundChannelController(unlocked);
        var stopped = new FakeBackgroundSession { State = "active", Temperature = 50, Rpm = 0, AgeMs = 100 };
        now = 0; running = true;
        while (running && now < 30000) { running = free.Step(stopped,now); now += 500; }
        Assert(running && free.Warning == "rpm_implausible" && !stopped.Closed,"Unrestricted plausibility deviation was not only a warning.");
        stopped.Temperature = 10;
        free.Step(stopped,now);
        Assert(stopped.Duties[stopped.Duties.Count-1] == 0,"Unrestricted channel could not reach 0%.");

        var ended = new FakeBackgroundSession { State = "emergency", Reason = "lease_expired" };
        Assert(!new BackgroundChannelController(config).Step(ended,0) && ended.Duties.Count == 0,"Terminal worker state was driven further.");
    }
}
