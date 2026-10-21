// Administrator-triggered fixed100 diagnostic. No public pipe setter or curve API.
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Web.Script.Serialization;

internal sealed class NativeProbeJob
{
    internal readonly string Id, ControlId;
    private readonly DateTime expires;
    private NativeProbeJob(string id, string control, DateTime expires) { Id=id; ControlId=control; this.expires=expires; }
    private static string Root { get { return Path.GetDirectoryName(BrokerTransport.DirectoryPath); } }
    internal static NativeProbeJob Claim()
    {
        BrokerService.CheckInstallation(false);
        string path=Path.Combine(Root,"mainboard-probe-request.json");
        BrokerService.CheckProtectedPath(path);
        if (new FileInfo(path).Length > 2048) throw new InvalidDataException("Oversized probe request.");
        var json=new JavaScriptSerializer { MaxJsonLength=2048,RecursionLimit=4 };
        var value=json.DeserializeObject(File.ReadAllText(path,Encoding.UTF8)) as Dictionary<string,object>;
        Guid id; DateTime expiry;
        if (value == null || value.Count != 4 || !value.ContainsKey("version") || !Object.Equals(value["version"],1) ||
            !value.ContainsKey("id") || !(value["id"] is string) || !Guid.TryParseExact((string)value["id"],"N",out id) ||
            !value.ContainsKey("control_id") || !(value["control_id"] is string) || !System.Text.RegularExpressions.Regex.IsMatch((string)value["control_id"],"^/lpc/[a-z0-9]+/[0-9]+/control/[0-6]$") ||
            !value.ContainsKey("expires_utc") || !(value["expires_utc"] is string) || !DateTime.TryParse((string)value["expires_utc"],System.Globalization.CultureInfo.InvariantCulture,System.Globalization.DateTimeStyles.RoundtripKind,out expiry) ||
            expiry.Kind != DateTimeKind.Utc || expiry <= DateTime.UtcNow || expiry > DateTime.UtcNow.AddSeconds(30))
            throw new InvalidDataException("Invalid or expired fixed100 diagnostic request.");
        File.Delete(path); // Consumed once; no automatic replay after restart.
        return new NativeProbeJob((string)value["id"],(string)value["control_id"],expiry);
    }
    internal bool Expired { get { return DateTime.UtcNow >= expires; } }
    internal void Complete(string reply)
    {
        string path=Path.Combine(Root,"mainboard-probe-result.json");
        BrokerService.CheckProtectedPath(Root);
        if (File.Exists(path)) BrokerService.CheckProtectedPath(path);
        var json=new JavaScriptSerializer { MaxJsonLength=262144,RecursionLimit=16 };
        object result=json.DeserializeObject(reply);
        string temporary=Path.Combine(Root,"mainboard-probe-result-"+Id+".tmp");
        if (File.Exists(temporary) || Directory.Exists(temporary)) throw new IOException("Probe result staging path exists.");
        try
        {
            File.WriteAllText(temporary,json.Serialize(new { version=1,id=Id,result=result }),new UTF8Encoding(false));
            BrokerService.CheckProtectedPath(temporary);
            if (File.Exists(path)) File.Replace(temporary,path,null); else File.Move(temporary,path);
        }
        finally { if (File.Exists(temporary)) File.Delete(temporary); }
    }
}
