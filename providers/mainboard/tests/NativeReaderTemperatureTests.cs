// Synthetic snapshots and ports only; no LHM Computer or installed service.
using System;
using System.Web.Script.Serialization;

internal static class NativeReaderTemperatureTests
{
    private const string Cpu = "/amdcpu/0/temperature/2", Gpu = "/gpu-nvidia/0/temperature/0";
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    private static string Snapshot(long acquired, object value)
    {
        return new JavaScriptSerializer().Serialize(new { version=1,status="ready",hardware=new[] {
            new { sensors=new[] { new { id=Cpu,kind="temperature",value=value,acquired_ms=acquired } } }
        } });
    }
    private static void Reject(string text, string[] sources, long now)
    {
        try { new NativeReaderTemperatures(text,sources,now); }
        catch (System.IO.InvalidDataException) { return; }
        throw new Exception("Invalid shared telemetry was accepted.");
    }
    internal static void Run()
    {
        var sources = new[] { Cpu };
        string snapshot = Snapshot(1000,45);
        var first = new NativeReaderTemperatures(snapshot,sources,2000);
        var cached = new NativeReaderTemperatures(snapshot,sources,4000);
        Assert(first.Read(Cpu) == 45 && first.AcquiredMs == 1000 && cached.AcquiredMs == 1000,"Cached read renewed acquisition time.");
        Reject(Snapshot(4001,45),sources,4000);
        Reject(Snapshot(0,45),sources,7001);
        Reject(Snapshot(1000,null),sources,2000);
        Reject(Snapshot(1000,151),sources,2000);
        Reject(snapshot,new[] { Cpu,Gpu },2000);
        Reject(snapshot.Replace("ready","faulted"),sources,2000);
        Reject(snapshot.Replace("acquired_ms","unknown"),sources,2000);

        // An eight-second startup has no session yet. The first checked sample
        // permits acquisition; subsequent healthy commands cannot renew CPU age.
        long now=8000;
        var guard=new NativeTelemetryGuard(80);
        guard.ObserveSharedTemperature(now,now,now,45,900);
        var port=new SessionPort { Floor=40 };
        var session=NativeControlSession.ForTest(port,delegate { return now; },guard,"lhm_supported",true);
        var view=session.View();
        for (now=8200; now<=15000; now+=200)
        {
            guard.ObserveSharedTemperature(now,now,8000,45,900);
            view=session.Execute(NativeSessionTests.Command(view,"set",50));
            Assert((string)view["state"] == "active","Slow startup or cached CPU reading stopped a fresh session.");
        }
        now=15001;
        Assert(!session.Poll(),"Frozen CPU temperature did not stop with fresh RPM and commands.");
        view=session.View();
        Assert((string)view["reason"] == "telemetry_stale" && !port.Owned && port.Writes[port.Writes.Length-2] == 100 && port.Writes[port.Writes.Length-1] == -1,"Stale shared source did not request 100% and owned firmware return.");

        guard=new NativeTelemetryGuard(80);
        guard.ObserveSharedTemperature(1000,1000,900,45,900);
        guard.ObserveSharedTemperature(1050,1050,900,50,901);
        var published=new JavaScriptSerializer().DeserializeObject(new JavaScriptSerializer().Serialize(guard.Snapshot(1050))) as System.Collections.Generic.Dictionary<string,object>;
        Assert(Convert.ToDouble(published["temperature_c"]) == 45 && Convert.ToDouble(published["rpm"]) == 901,"Mixed source changed a cached temperature identity or hid new RPM.");
        guard.ObserveSharedTemperature(1100,1100,900,85,900);
        Assert(guard.Check(1100) == "temperature_critical","Cached oldest timestamp hid critical heat in a mixed source.");
        guard=new NativeTelemetryGuard(80);
        guard.ObserveSharedTemperature(1000,1000,900,45,0);
        guard.ObserveSharedTemperature(2000,2000,900,45,0);
        Assert(guard.Check(2000) == "fan_stalled","Shared CPU temperature delayed independent RPM stall detection.");
        Console.WriteLine("PASS: shared reader acquisition identity, bounded startup, frozen CPU with fresh RPM, critical heat and owned firmware return; synthetic only.");
    }
}
