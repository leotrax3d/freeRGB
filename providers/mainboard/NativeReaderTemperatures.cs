// CPU/GPU telemetry has one LHM owner: the inventory reader. The caller must
// authenticate the broker transport before passing a snapshot to this decoder.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Web.Script.Serialization;

internal sealed class NativeReaderTemperatures
{
    internal static long Now { get { return (long)(Stopwatch.GetTimestamp() * 1000.0 / Stopwatch.Frequency); } }
    internal readonly long AcquiredMs;
    private readonly Dictionary<string,double> values = new Dictionary<string,double>();

    internal NativeReaderTemperatures(string snapshot, string[] requested, long now)
    {
        var root = new JavaScriptSerializer { MaxJsonLength=BrokerTransport.Limit,RecursionLimit=32 }.DeserializeObject(snapshot) as Dictionary<string,object>;
        if (root == null || !Object.Equals(root["version"],1) || !Object.Equals(root["status"],"ready") || !(root["hardware"] is object[]))
            throw new InvalidDataException("Trusted temperature reader is unavailable.");
        AcquiredMs = now;
        var wanted = new HashSet<string>();
        foreach (string id in requested) if (!id.StartsWith("/lpc/",StringComparison.Ordinal)) wanted.Add(id);
        foreach (Dictionary<string,object> hardware in (object[])root["hardware"])
            foreach (Dictionary<string,object> sensor in (object[])hardware["sensors"])
            {
                string id = sensor["id"] as string;
                if (id == null || !wanted.Contains(id)) continue;
                object acquired;
                if (!Object.Equals(sensor["kind"],"temperature") || sensor["value"] == null ||
                    !sensor.TryGetValue("acquired_ms",out acquired) || (!(acquired is int) && !(acquired is long)))
                    throw new InvalidDataException("Trusted temperature lacks acquisition evidence.");
                long at = Convert.ToInt64(acquired);
                double value = Convert.ToDouble(sensor["value"]);
                if (at < 0 || at > now || now - at > NativeTelemetryGuard.MaxAgeMs ||
                    Double.IsNaN(value) || Double.IsInfinity(value) || value < -20 || value > 150 || values.ContainsKey(id))
                    throw new InvalidDataException("Invalid trusted temperature.");
                values.Add(id,value);
                AcquiredMs = Math.Min(AcquiredMs,at);
            }
        if (values.Count != wanted.Count) throw new InvalidDataException("Selected trusted temperature is missing.");
    }
    internal double Read(string id) { return values[id]; }
}
