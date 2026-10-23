// Support-tier tests (verified / lhm_supported / unrestricted). Real pinned LHM
// Control objects with synthetic callbacks; temporary journals; no hardware.
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;

internal static class NativeTierTests
{
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action, string message)
    {
        try { action(); }
        catch (InvalidOperationException) { return; } catch (ArgumentException) { return; } catch (InvalidDataException) { return; }
        catch (AggregateException) { return; } catch (TargetInvocationException) { return; }
        throw new Exception(message);
    }
    private static Dictionary<string,object> Fallback()
    {
        return new Dictionary<string,object> { {"kind","lhm_fallback_v1"},{"service_identity","123:456"},{"worker_identity","789:1011"} };
    }
    internal static void Run(Func<ControlSink,object> create)
    {
        Adapters(create);
        Rules();
        Journal();
        Sessions(create);
        Telemetry();
        Console.WriteLine("PASS: tiered admission (lhm_supported floor 40, unrestricted 0% and pump unlock), authorization rules, tier journal/fallback metadata, owned fault return and stall warning. Synthetic only.");
    }
    private static void Adapters(Func<ControlSink,object> create)
    {
        var sink = new ControlSink();
        var lhm = NativeControl.ForTest(create(sink),false,40,NativeTier.LhmSupported,false,null);
        Assert(lhm.Tier == NativeTier.LhmSupported && !lhm.RegisterReadback,"Missing readback was not reported as lhm_supported.");
        lhm.Acquire();
        Assert(sink.Writes.Count == 1 && sink.Writes[0] == 100,"LHM-supported acquisition did not start at a prepared 100%.");
        lhm.SetDuty(40);
        Reject(delegate { lhm.SetDuty(39); },"LHM-supported channel accepted a duty below 40%.");
        Reject(delegate { lhm.SetDuty(0); },"LHM-supported channel accepted fan stop.");
        var identity = lhm.Identity();
        Assert(identity.Tier == "lhm_supported" && identity.Recovery == null && !identity.Pump,"Tier missing from journal identity.");
        lhm.Release();
        Assert(lhm.FirmwareReturned && !lhm.FirmwareReturnVerified && sink.Writes[sink.Writes.Count-1] == null,
            "Callback-level return was reported as register-verified or did not reach firmware.");

        sink = new ControlSink();
        var unlocked = NativeControl.ForTest(create(sink),false,0,NativeTier.Unrestricted,false,null);
        unlocked.Acquire();
        unlocked.SetDuty(0);
        Assert(sink.Writes.Count == 2 && sink.Writes[0] == 100 && sink.Writes[1] == 0,"Unrestricted fan stop was not written exactly once after 100%.");
        foreach (float bad in new[] { -1f,101f,Single.NaN }) Reject(delegate { unlocked.SetDuty(bad); },"Invalid unrestricted duty accepted.");
        Assert(unlocked.Identity().Tier == "unrestricted","Unrestricted tier missing from identity.");
        var recorded = NativeControl.ForTest(create(new ControlSink()),false,0,NativeTier.Unrestricted,false,null,Fallback()).Identity();
        Assert(recorded.Recovery != null && (string)recorded.Recovery["kind"] == "lhm_fallback_v1","Fallback lifetime metadata missing from identity.");
        unlocked.Release();

        foreach (var invalid in new Action[] {
            delegate { NativeControl.ForTest(create(new ControlSink()),false,0,NativeTier.LhmSupported,false,null); },
            delegate { NativeControl.ForTest(create(new ControlSink()),false,39,NativeTier.Verified,false,null); },
            delegate { NativeControl.ForTest(create(new ControlSink()),true,30,NativeTier.LhmSupported,true,null); },
            delegate { NativeControl.ForTest(create(new ControlSink()),false,101,NativeTier.Unrestricted,false,null); } })
            Reject(invalid,"Unsafe tier/floor combination constructed an adapter.");

        sink = new ControlSink();
        var pump = NativeControl.ForTest(create(sink),true,0,NativeTier.Unrestricted,false,null);
        pump.Acquire();
        Reject(delegate { pump.SetDuty(60); },"Unrestricted pump without the second confirmation left 100%.");
        Assert(pump.Identity().Pump,"Pump role lost from identity.");
        pump.Release();
        sink = new ControlSink();
        var confirmedPump = NativeControl.ForTest(create(sink),true,30,NativeTier.Unrestricted,true,null);
        confirmedPump.Acquire(); confirmedPump.SetDuty(30);
        Assert(confirmedPump.Identity().Pump && sink.Writes[1] == 30,"Separately confirmed pump reduction failed or lost its pump role.");
        confirmedPump.Release();

        var foreignSink = new ControlSink();
        object foreign = create(foreignSink);
        foreign.GetType().GetMethod("SetSoftware").Invoke(foreign,new object[] { 100f });
        int before = foreignSink.Writes.Count;
        Reject(NativeControl.ForTest(foreign,false,0,NativeTier.Unrestricted,false,null).Acquire,"Unrestricted unlock took over a channel already in software mode.");
        Assert(foreignSink.Writes.Count == before,"Refused foreign takeover performed I/O.");

        var proof = new EvidenceSink();
        var readback = NativeControl.ForTest(create(new ControlSink()),false,0,NativeTier.Unrestricted,false,proof);
        Assert(readback.Tier == NativeTier.Unrestricted && readback.RegisterReadback,"Unlocked audited chip dropped register readback.");
        readback.Acquire(); readback.SetDuty(0);
        Assert(proof.DutyChecks == 2,"Register readback skipped for an unlocked audited chip.");
        readback.Release();
        Assert(readback.FirmwareReturnVerified && proof.ReturnChecks == 1,"Unlocked audited chip skipped verified return.");
    }
    private static Dictionary<string,object> Record(params object[] pairs)
    {
        var value = new Dictionary<string,object>();
        for (int i = 0; i < pairs.Length; i += 2) value.Add((string)pairs[i],pairs[i+1]);
        return value;
    }
    private static void Rules()
    {
        NativeTier tier; bool pump, unlocked;
        NativeAuthorizationRules.Validate(Record("role","fan"),40,out tier,out pump,out unlocked);
        Assert(tier == NativeTier.Verified && !pump && !unlocked,"Legacy fan record changed meaning.");
        NativeAuthorizationRules.Validate(Record(),100,out tier,out pump,out unlocked);
        NativeAuthorizationRules.Validate(Record("role","fan","tier","lhm_supported"),40,out tier,out pump,out unlocked);
        Assert(tier == NativeTier.LhmSupported,"LHM-supported record not parsed.");
        NativeAuthorizationRules.Validate(Record("role","fan","tier","unrestricted"),0,out tier,out pump,out unlocked);
        Assert(tier == NativeTier.Unrestricted,"Unrestricted record not parsed.");
        NativeAuthorizationRules.Validate(Record("role","pump","tier","unrestricted","pump_unlocked",true),20,out tier,out pump,out unlocked);
        Assert(pump && unlocked,"Confirmed pump unlock not parsed.");
        NativeAuthorizationRules.Validate(Record("role","pump","tier","lhm_supported"),100,out tier,out pump,out unlocked);
        foreach (var bad in new object[][] {
            new object[] { Record("role","fan"),39 },
            new object[] { Record(),60 },
            new object[] { Record("role","pump"),60 },
            new object[] { Record("role","fan","tier","verified"),40 },
            new object[] { Record("role","fan","tier","lhm_supported"),0 },
            new object[] { Record("role","fan","tier","bogus"),40 },
            new object[] { Record("role","pump","tier","lhm_supported"),60 },
            new object[] { Record("role","pump","tier","unrestricted"),60 },
            new object[] { Record("role","pump","tier","lhm_supported","pump_unlocked",true),100 },
            new object[] { Record("role","fan","tier","unrestricted","pump_unlocked",true),20 },
            new object[] { Record("role","pump","tier","unrestricted","pump_unlocked",false),100 },
            new object[] { Record("role","heater","tier","unrestricted"),40 },
            new object[] { Record("role","fan","tier","unrestricted"),101 } })
        {
            var value = (Dictionary<string,object>)bad[0]; int minimum = (int)bad[1];
            Reject(delegate { NativeTier t; bool p, u; NativeAuthorizationRules.Validate(value,minimum,out t,out p,out u); },
                "Unsafe authorization accepted: " + new System.Web.Script.Serialization.JavaScriptSerializer().Serialize(value) + " @" + minimum);
        }
    }
    private static void Journal()
    {
        string root = Path.Combine(Path.GetTempPath(),"freergb-tier-journal-"+Guid.NewGuid().ToString("N"));
        string path = Path.Combine(root,"mainboard-control-recovery.json");
        try
        {
            using (var journal = NativeControlJournal.ForTest(root))
            {
                string id = journal.Reserve(new NativeControlIdentity("/lpc/it8689e/0","/lpc/it8689e/0/control/1",false,Fallback(),"unrestricted"));
                var pending = journal.Pending();
                Assert((string)pending["tier"] == "unrestricted" && (string)((Dictionary<string,object>)pending["recovery"])["kind"] == "lhm_fallback_v1",
                    "Tier/fallback metadata was not persisted before I/O.");
                journal.Complete(id);
                Assert(File.ReadAllText(path) == "{\"version\":1,\"reservation\":null}","Cleared journal changed its legacy empty form.");
                id = journal.Reserve(new NativeControlIdentity("/lpc/nct6701d/0","/lpc/nct6701d/0/control/2",false));
                Assert(!journal.Pending().ContainsKey("tier"),"Verified reservation gained a tier field.");
                journal.Complete(id);
                Reject(delegate { journal.Reserve(new NativeControlIdentity("/a","/a/control/1",false,Fallback())); },"Fallback recovery without a lower tier accepted.");
                Reject(delegate { journal.Reserve(new NativeControlIdentity("/a","/a/control/1",false,null,"verified")); },"Explicit verified tier field accepted.");
                var badLifetime = Fallback(); badLifetime["worker_identity"] = "0:abc";
                Reject(delegate { journal.Reserve(new NativeControlIdentity("/a","/a/control/1",false,badLifetime,"lhm_supported")); },"Invalid fallback lifetime accepted.");
                Assert(journal.Pending() == null,"Rejected identities left a pending intent.");
                foreach (string invalid in new[] {
                    "{\"version\":1,\"reservation\":{\"id\":\"0123456789abcdef0123456789abcdef\",\"backend\":\"" + LhmPin.Backend + "\",\"hardware_id\":\"/a\",\"sensor_id\":\"/a/c\",\"pump\":false,\"tier\":\"verified\"}}",
                    "{\"version\":1,\"reservation\":{\"id\":\"0123456789abcdef0123456789abcdef\",\"backend\":\"" + LhmPin.Backend + "\",\"hardware_id\":\"/a\",\"sensor_id\":\"/a/c\",\"pump\":false,\"recovery\":{\"kind\":\"lhm_fallback_v1\",\"service_identity\":\"1:1\",\"worker_identity\":\"1:1\"}}}",
                    "{\"version\":1,\"reservation\":{\"id\":\"0123456789abcdef0123456789abcdef\",\"backend\":\"" + LhmPin.Backend + "\",\"hardware_id\":\"/a\",\"sensor_id\":\"/a/c\",\"pump\":false,\"tier\":\"lhm_supported\",\"extra\":1}}" })
                {
                    File.WriteAllText(path,invalid);
                    Reject(delegate { journal.Pending(); },"Invalid tiered journal accepted.");
                    Reject(delegate { journal.Reserve(new NativeControlIdentity("/b","/b/control/1",false)); },"Invalid tiered journal permitted acquisition.");
                }
                File.WriteAllText(path,"{\"version\":1,\"reservation\":null}");
            }
        }
        finally { if (Directory.Exists(root)) Directory.Delete(root,true); }
    }
    private static void Sessions(Func<ControlSink,object> create)
    {
        string root = Path.Combine(Path.GetTempPath(),"freergb-tier-session-"+Guid.NewGuid().ToString("N"));
        try
        {
            using (var journal = NativeControlJournal.ForTest(root))
            {
                foreach (bool unrestricted in new[] { false, true })
                {
                    var sink = new ControlSink();
                    var adapter = NativeControl.ForTest(create(sink),false,unrestricted ? 0 : 40,unrestricted ? NativeTier.Unrestricted : NativeTier.LhmSupported,false,null);
                    var wrapped = JournaledNativeControl.ForTest(adapter,journal,adapter.Identity());
                    long now = 0;
                    var session = NativeControlSession.ForTest(wrapped,delegate { return now; },null,NativeTiers.Name(adapter.Tier),true);
                    var view = session.View();
                    Assert((string)view["tier"] == NativeTiers.Name(adapter.Tier) && !(bool)view["firmware_returned"] && !(bool)view["journal_cleared"],"Active tiered receipt claimed return evidence.");
                    session.Execute(NativeSessionTests.Command(view,"set",unrestricted ? 0 : 40));
                    Assert((int)session.View()["duty_percent"] == (unrestricted ? 0 : 40),"Tiered duty not applied.");
                    now = 1000; session.Poll(); // Lease (watchdog) expiry.
                    view = session.View();
                    Assert((string)view["state"] == "emergency" && (string)view["reason"] == "lease_expired" && !(bool)view["owned"],"Tiered lease expiry did not end ownership.");
                    Assert((bool)view["firmware_returned"] && (bool)view["journal_cleared"] && journal.Pending() == null,"Tiered fault return did not clear the journal after the owned firmware callback.");
                    int n = sink.Writes.Count;
                    Assert(sink.Writes[n-2] == 100 && sink.Writes[n-1] == null,"Fault did not command 100% before returning to the captured firmware mode.");
                }
                // Legacy (verified) sessions keep holding 100% for external recovery.
                var legacySink = new ControlSink();
                var legacy = NativeControl.ForTest(create(legacySink),false);
                var legacyWrapped = JournaledNativeControl.ForTest(legacy,journal,legacy.Identity());
                long at = 0;
                var held = NativeControlSession.ForTest(legacyWrapped,delegate { return at; });
                at = 1000; held.Poll();
                Assert(legacy.Owned && journal.Pending() != null && !held.View().ContainsKey("tier"),"Verified fault behaviour changed.");
                legacyWrapped.Release();
                // A failed tiered return keeps the intent for recovery.
                var failingSink = new ControlSink();
                var failing = NativeControl.ForTest(create(failingSink),false,40,NativeTier.LhmSupported,false,null);
                var failingWrapped = JournaledNativeControl.ForTest(failing,journal,failing.Identity());
                long t = 0;
                var failingSession = NativeControlSession.ForTest(failingWrapped,delegate { return t; },null,"lhm_supported",true);
                failingSink.FailDefault = true;
                t = 1000; failingSession.Poll();
                var failed = failingSession.View();
                Assert((string)failed["state"] == "recovery_failed" && (bool)failed["owned"] && journal.Pending() != null && !(bool)failed["journal_cleared"],
                    "Failed tiered firmware return cleared ownership or journal.");
                failingSink.FailDefault = false;
                failingWrapped.Release();
                Assert(journal.Pending() == null,"Explicit retry did not clear the retained intent.");
                Reject(delegate { NativeControlSession.ForTest(new SessionPort { Floor = 40 },delegate { return 0; },null,null,true); },"Fault return without tier accepted.");
            }
        }
        finally { if (Directory.Exists(root)) Directory.Delete(root,true); }
    }
    private static void Telemetry()
    {
        var warn = new NativeTelemetryGuard(80,true);
        warn.Observe(0,0,40,0);
        warn.Observe(1200,1200,40,0);
        Assert(warn.Check(1300) == null && warn.Warning == "fan_stalled","Unrestricted fan stop faulted instead of warning.");
        Assert(warn.Check(2800) == "telemetry_stale","Stall warning masked stale telemetry.");
        var hot = new NativeTelemetryGuard(80,true);
        hot.Observe(0,0,80,0);
        Assert(hot.Check(10) == "temperature_critical","Stall warning masked the critical temperature override.");
        var strict = new NativeTelemetryGuard(80,false);
        strict.Observe(0,0,40,0); strict.Observe(1200,1200,40,0);
        Assert(strict.Check(1300) == "fan_stalled" && strict.Warning == null,"Default guard no longer faults on a stalled fan.");
    }
}
