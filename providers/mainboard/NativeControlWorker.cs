// Protected native boundary: exact sensor binding, independent checked sampler,
// authenticated service-only transport. Curves/calibration remain in Rust.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Security.Principal;
using System.Threading;
using System.Web.Script.Serialization;

#if NATIVE_CONTROL_WORKER
internal static class NativeControlWorker
{
    internal static int Run(object control, string pipeName, string encoded, Func<string,object> findTemperature)
    {
        BrokerService.CheckInstallation(true);
        if (!System.Text.RegularExpressions.Regex.IsMatch(pipeName,"^freergb-control-worker-[a-f0-9]{32}$")) throw new InvalidDataException("Invalid private pipe name.");
        if (encoded.Length > 4096) throw new InvalidDataException("Control binding too large.");
        var binding=new JavaScriptSerializer { MaxJsonLength=2048,RecursionLimit=8 }.DeserializeObject(
            new System.Text.UTF8Encoding(false,true).GetString(Convert.FromBase64String(encoded))) as Dictionary<string,object>;
        int optional = binding == null ? 0 : (binding.ContainsKey("role") ? 1 : 0) + (binding.ContainsKey("tier") ? 1 : 0) + (binding.ContainsKey("pump_unlocked") ? 1 : 0);
        if (binding == null || binding.Count != 5 + optional || !Object.Equals(binding["version"],1) ||
            !(binding["temperature_id"] is string) || !(binding["rpm_id"] is string) ||
            !(binding["floor_percent"] is int) || !(binding["critical_c"] is int))
            throw new InvalidDataException("Invalid native commissioning binding.");
        int minimum=(int)binding["floor_percent"];
        NativeTier tier; bool pump, pumpUnlocked;
        NativeAuthorizationRules.Validate(binding,minimum,out tier,out pump,out pumpUnlocked);
        string temperatureId=(string)binding["temperature_id"], rpmId=(string)binding["rpm_id"];
        if (temperatureId.Length > 512 || rpmId.Length > 512) throw new InvalidDataException("Sensor identity too large.");
        // One SuperIO source keeps the original (verified) path; otherwise CPU/GPU
        // and mixed (hottest-of) sources are read by NativeIsa.ReadSources.
        string[] temperatures = NativeSources.Temperatures(temperatureId);
        bool single = temperatures.Length == 1 && temperatures[0].StartsWith("/lpc/",StringComparison.Ordinal);
        bool legacy = !binding.ContainsKey("tier");
        // The legacy/verified path is unchanged: register evidence is mandatory.
        var native=legacy ? NativeControl.CreateCommissioningProtected(control,minimum) : NativeControl.CreateTieredProtected(control,pump,minimum,tier,pumpUnlocked);
        var evidence=legacy ? NativeNctEvidence.Create(control) : null;
        var guard=new NativeTelemetryGuard((int)binding["critical_c"],!legacy && native.Tier == NativeTier.Unrestricted);
        var clock=Stopwatch.StartNew();
        Action sample=delegate {
            long acquired=clock.ElapsedMilliseconds;
            double celsius,rpm;
            if (single && evidence != null) evidence.ReadTelemetry(control,temperatureId,rpmId,out celsius,out rpm);
            else if (single) NativeIsa.ReadTelemetry(control,temperatureId,rpmId,out celsius,out rpm);
            else NativeIsa.ReadSources(control,temperatures,rpmId,findTemperature,out celsius,out rpm);
            guard.Observe(clock.ElapsedMilliseconds,acquired,celsius,rpm);
        };
        sample(); // Validated fresh data before any acquisition.
        using (var stop=new ManualResetEvent(false))
        using (var journal=NativeControlJournal.CreateProtected())
        using (var pipe=BrokerTransport.CreateServer(pipeName,new SecurityIdentifier(WellKnownSidType.LocalSystemSid,null)))
        {
            var sampler=new Thread(delegate() {
                try { while (!stop.WaitOne(100)) sample(); }
                catch { /* No freshness renewal: the independent guard expires. */ }
            }) { IsBackground=true,Priority=ThreadPriority.AboveNormal,Name="freeRGB checked native telemetry" };
            sampler.Start();
            try
            {
                Console.WriteLine("ready"); Console.Out.Flush();
                pipe.WaitForConnection();
                if (legacy) NativeSessionTransport.RunProtected(pipe,JournaledNativeControl.CreateProtected(native,journal),guard,delegate { return clock.ElapsedMilliseconds; });
                else NativeSessionTransport.RunTieredProtected(pipe,JournaledNativeControl.CreateProtected(native,journal),guard,delegate { return clock.ElapsedMilliseconds; },native.Tier);
                return 0;
            }
            finally
            {
                stop.Set();
                if (!sampler.Join(1500)) throw new InvalidOperationException("Native sampler did not stop; preserve recovery evidence.");
            }
        }
    }
}
#endif
