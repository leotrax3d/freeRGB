using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Text;
using System.Web.Script.Serialization;

internal sealed class BrokerCache
{
    private readonly object gate = new object();
    private string snapshot;
    private long sampled;
    internal static long Now { get { return Stopwatch.GetTimestamp(); } }
    internal static string Failure(string message)
    {
        return new JavaScriptSerializer().Serialize(new { version = 1, status = "faulted", message = message, hardware = new object[0] });
    }
    private static Dictionary<string, object> Parse(string text)
    {
        if (Encoding.UTF8.GetByteCount(text) >= BrokerTransport.Limit) throw new ArgumentException("Snapshot too large.");
        var json = new JavaScriptSerializer { MaxJsonLength = BrokerTransport.Limit, RecursionLimit = 32 };
        var result = json.DeserializeObject(text) as Dictionary<string, object>;
        if (result == null || !result.ContainsKey("version") || !Object.Equals(result["version"], 1) ||
            !result.ContainsKey("status") || !result.ContainsKey("hardware") || !(result["hardware"] is object[]))
            throw new ArgumentException("Invalid snapshot envelope.");
        string status = result["status"] as string;
        if (Array.IndexOf(new[] { "ready", "missing_library", "missing_driver", "permission_denied", "no_supported_hardware", "faulted" }, status) < 0)
            throw new ArgumentException("Unknown snapshot status.");
        var hardware = (object[])result["hardware"];
        if (hardware.Length > 64) throw new ArgumentException("Hardware limit exceeded.");
        int count = 0;
        foreach (object entry in hardware)
        {
            var device = entry as Dictionary<string, object>;
            if (device == null || !device.ContainsKey("sensors") || !(device["sensors"] is object[])) throw new ArgumentException("Invalid hardware entry.");
            foreach (object item in (object[])device["sensors"])
            {
                if (++count > 1024) throw new ArgumentException("Sensor limit exceeded.");
                var sensor = item as Dictionary<string, object>;
                if (sensor == null || !sensor.ContainsKey("age_ms") || !sensor.ContainsKey("value")) throw new ArgumentException("Invalid sensor entry.");
                object age = sensor["age_ms"];
                if (!(age is int) && !(age is long)) throw new ArgumentException("Invalid sensor age.");
                if (Convert.ToInt64(age) < 0) throw new ArgumentException("Negative sensor age.");
            }
        }
        return result;
    }
    internal void Store(string text, long requestStarted)
    {
        Parse(text);
        long now = Now;
        if (requestStarted < 0 || requestStarted > now) throw new ArgumentException("Invalid sample clock.");
        lock (gate) { snapshot = text; sampled = requestStarted; }
    }
    internal string Read()
    {
        string text; long start;
        lock (gate) { text = snapshot; start = sampled; }
        if (text == null) return Failure("Motherboard reader is starting.");
        var result = Parse(text);
        long elapsed = (long)((Now - start) * 1000.0 / Stopwatch.Frequency);
        // Never refresh measurement age when relaying an old sample.
        if (elapsed >= 5000) return Failure("Motherboard reader sample expired.");
        foreach (Dictionary<string, object> device in (IEnumerable)result["hardware"])
            foreach (Dictionary<string, object> sensor in (IEnumerable)device["sensors"])
            {
                long age = Convert.ToInt64(sensor["age_ms"]);
                sensor["age_ms"] = age > Int64.MaxValue - elapsed ? Int64.MaxValue : age + elapsed;
                if ((long)sensor["age_ms"] >= 5000) sensor["value"] = null;
            }
        string response = new JavaScriptSerializer { MaxJsonLength = BrokerTransport.Limit }.Serialize(result);
        return Encoding.UTF8.GetByteCount(response) < BrokerTransport.Limit ? response : Failure("Motherboard reply exceeds its size limit.");
    }
}
