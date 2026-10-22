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
    internal static int Run(object control, string pipeName, string encoded)
    {
        BrokerService.CheckInstallation(true);
        if (!System.Text.RegularExpressions.Regex.IsMatch(pipeName,"^freergb-control-worker-[a-f0-9]{32}$")) throw new InvalidDataException("Invalid private pipe name.");
        if (encoded.Length > 4096) throw new InvalidDataException("Control binding too large.");
        var binding=new JavaScriptSerializer { MaxJsonLength=2048,RecursionLimit=8 }.DeserializeObject(
            new System.Text.UTF8Encoding(false,true).GetString(Convert.FromBase64String(encoded))) as Dictionary<string,object>;
        if (binding == null || (binding.Count != 5 && binding.Count != 6) || !Object.Equals(binding["version"],1) ||
            !(binding["temperature_id"] is string) || !(binding["rpm_id"] is string) ||
            !(binding["floor_percent"] is int) || !(binding["critical_c"] is int))
            throw new InvalidDataException("Invalid native commissioning binding.");
        int minimum=(int)binding["floor_percent"];
        if ((minimum < 40 || minimum > 100) ||
            (minimum < 100 && (!binding.ContainsKey("role") || !Object.Equals(binding["role"],"fan"))))
            throw new InvalidDataException("Reduced commissioning duty requires explicit protected fan authorization.");
        string temperatureId=(string)binding["temperature_id"], rpmId=(string)binding["rpm_id"];
        if (temperatureId.Length > 512 || rpmId.Length > 512) throw new InvalidDataException("Sensor identity too large.");
        var native=NativeControl.CreateCommissioningProtected(control,minimum);
        var evidence=NativeNctEvidence.Create(control);
        var guard=new NativeTelemetryGuard((int)binding["critical_c"]);
        var clock=Stopwatch.StartNew();
        Action sample=delegate {
            long acquired=clock.ElapsedMilliseconds;
            double celsius,rpm;
            evidence.ReadTelemetry(control,temperatureId,rpmId,out celsius,out rpm);
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
            }) { IsBackground=true,Name="freeRGB checked native telemetry" };
            sampler.Start();
            try
            {
                Console.WriteLine("ready"); Console.Out.Flush();
                pipe.WaitForConnection();
                NativeSessionTransport.RunProtected(pipe,JournaledNativeControl.CreateProtected(native,journal),guard,delegate { return clock.ElapsedMilliseconds; });
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
