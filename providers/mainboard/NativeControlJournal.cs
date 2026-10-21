// Protected write-ahead intent. No automatic replay/recovery of old fan duties.
using System;
using System.Collections.Generic;
using System.IO;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

internal sealed class NativeControlIdentity
{
    internal readonly string HardwareId, SensorId;
    internal readonly bool Pump;
    internal readonly Dictionary<string,object> Recovery;
    internal NativeControlIdentity(string hardware, string sensor, bool pump, Dictionary<string,object> recovery = null)
    { HardwareId = hardware; SensorId = sensor; Pump = pump; Recovery = recovery; }
}
internal sealed class NativeControlJournal : IDisposable
{
    private readonly string root, path;
    private readonly Mutex mutex;
    private const int Limit = 16384;
    private NativeControlJournal(string root, bool fixture, bool maintenance = false)
    {
        this.root = Path.GetFullPath(root);
        path = Path.Combine(this.root, "mainboard-control-recovery.json");
        string name = @"Global\freeRGB.Mainboard.ControlJournal.v1";
#if NATIVE_CONTROL_TESTS
        if (fixture)
        {
            using (var hash = SHA256.Create()) name = @"Local\freeRGB.JournalFixture." + BitConverter.ToString(hash.ComputeHash(Encoding.UTF8.GetBytes(path.ToUpperInvariant()))).Replace("-", "");
            mutex = new Mutex(false,name);
            return;
        }
#endif
        var system = new SecurityIdentifier(WellKnownSidType.LocalSystemSid,null);
        var admins = new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid,null);
        var acl = new MutexSecurity(); acl.SetAccessRuleProtection(true,false); acl.SetOwner(maintenance ? admins : system);
        acl.AddAccessRule(new MutexAccessRule(system,MutexRights.FullControl,AccessControlType.Allow));
        acl.AddAccessRule(new MutexAccessRule(admins,MutexRights.FullControl,AccessControlType.Allow));
        bool created;
        mutex = new Mutex(false,name,out created,acl);
        var current = mutex.GetAccessControl();
        var owner = (SecurityIdentifier)current.GetOwner(typeof(SecurityIdentifier));
        if (!owner.Equals(system) && !owner.Equals(admins)) { mutex.Dispose(); throw new InvalidOperationException("Untrusted control-journal mutex owner."); }
        foreach (MutexAccessRule rule in current.GetAccessRules(true,true,typeof(SecurityIdentifier)))
            if (rule.AccessControlType == AccessControlType.Allow && !rule.IdentityReference.Equals(system) && !rule.IdentityReference.Equals(admins))
            { mutex.Dispose(); throw new InvalidOperationException("Control-journal mutex permits an unrelated account."); }
    }
#if NATIVE_CONTROL_TESTS
    internal static NativeControlJournal ForTest(string root) { Directory.CreateDirectory(root); return new NativeControlJournal(root,true); }
#else
    internal static NativeControlJournal CreateProtected()
    {
        BrokerService.CheckInstallation(true);
        return new NativeControlJournal(Path.GetDirectoryName(BrokerTransport.DirectoryPath),false);
    }
    internal static NativeControlJournal CreateForMaintenance()
    {
        using (var identity = WindowsIdentity.GetCurrent())
            if (!new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator))
                throw new UnauthorizedAccessException("Elevated broker maintenance required.");
        string root = Path.GetDirectoryName(BrokerTransport.DirectoryPath);
        BrokerService.CheckProtectedPath(root);
        return new NativeControlJournal(root,false,true);
    }
#endif
    private void Check(string file)
    {
#if !NATIVE_CONTROL_TESTS
        BrokerService.CheckProtectedPath(root);
        if (File.Exists(file) || Directory.Exists(file)) BrokerService.CheckProtectedPath(file);
#endif
    }
    private T Locked<T>(Func<T> action)
    {
        bool acquired = false;
        try
        {
            try { acquired = mutex.WaitOne(5000); } catch (AbandonedMutexException) { acquired = true; }
            if (!acquired) throw new IOException("Control-journal lock timed out; no native acquisition permitted.");
            return action();
        }
        finally { if (acquired) mutex.ReleaseMutex(); }
    }
    private static bool IdentityText(string value)
    {
        if (String.IsNullOrWhiteSpace(value) || Encoding.UTF8.GetByteCount(value) > 512) return false;
        foreach (char c in value) if (Char.IsControl(c)) return false;
        return true;
    }
    private Dictionary<string, object> Read()
    {
        Check(path);
        if (Directory.Exists(path)) throw new InvalidDataException("Control journal path is not a file.");
        if (!File.Exists(path)) return null;
        byte[] bytes;
        using (var stream = new FileStream(path,FileMode.Open,FileAccess.Read,FileShare.Read))
        {
            if (stream.Length > Limit) throw new InvalidDataException("Oversized control journal; recovery required.");
            bytes = new byte[checked((int)stream.Length)];
            int at = 0;
            while (at < bytes.Length) { int read = stream.Read(bytes,at,bytes.Length-at); if (read == 0) throw new EndOfStreamException(); at += read; }
        }
        var json = new JavaScriptSerializer { MaxJsonLength = Limit, RecursionLimit = 8 };
        Dictionary<string, object> document;
        try { document = json.DeserializeObject(new UTF8Encoding(false,true).GetString(bytes)) as Dictionary<string, object>; }
        catch (Exception error) { throw new InvalidDataException("Invalid control journal; recovery required.",error); }
        if (document == null || document.Count != 2 || !document.ContainsKey("version") || !Object.Equals(document["version"],1) || !document.ContainsKey("reservation"))
            throw new InvalidDataException("Unsupported control journal; recovery required.");
        if (document["reservation"] == null) return null;
        var entry = document["reservation"] as Dictionary<string, object>;
        Guid identifier;
        if (entry == null || (entry.Count != 5 && !(entry.Count == 6 && entry.ContainsKey("recovery"))) || !entry.ContainsKey("id") || !(entry["id"] is string) || !Guid.TryParseExact((string)entry["id"],"N",out identifier) ||
            !entry.ContainsKey("backend") || !Object.Equals(entry["backend"],"lhm_0_9_6") ||
            !entry.ContainsKey("hardware_id") || !(entry["hardware_id"] is string) || !IdentityText((string)entry["hardware_id"]) ||
            !entry.ContainsKey("sensor_id") || !(entry["sensor_id"] is string) || !IdentityText((string)entry["sensor_id"]) ||
            !entry.ContainsKey("pump") || !(entry["pump"] is bool))
            throw new InvalidDataException("Invalid control reservation; recovery required.");
        if (entry.ContainsKey("recovery")) ValidateRecovery(entry["recovery"] as Dictionary<string,object>);
        return entry;
    }
    private static void ValidateRecovery(Dictionary<string,object> value)
    {
        if (value == null || (value.Count != 7 && value.Count != 9) || !value.ContainsKey("kind") || !Object.Equals(value["kind"],"lhm_nct677x_registers_v1") ||
            !value.ContainsKey("chip") || !(value["chip"] is string) || !IdentityText((string)value["chip"]))
            throw new InvalidDataException("Invalid native recovery metadata.");
        foreach (string key in new[] { "index","port","revision","mode","command" })
            if (!value.ContainsKey(key) || !(value[key] is int) || (int)value[key] < 0 || (int)value[key] > (key == "index" ? 6 : key == "port" ? 65535 : 255))
                throw new InvalidDataException("Invalid native recovery register metadata.");
        if ((int)value["mode"] == 0) throw new InvalidDataException("Recovery metadata must describe original automatic firmware mode.");
        if (value.Count == 9)
            foreach (string key in new[] { "service_identity","worker_identity" })
                if (!value.ContainsKey(key) || !(value[key] is string) || !System.Text.RegularExpressions.Regex.IsMatch((string)value[key],"^[1-9][0-9]{0,9}:[1-9][0-9]{0,18}$"))
                    throw new InvalidDataException("Invalid native recovery lifetime metadata.");
    }
    private void Write(Dictionary<string, object> entry)
    {
        Check(path);
        string temporary = path + ".tmp"; Check(temporary);
        byte[] bytes = Encoding.UTF8.GetBytes(new JavaScriptSerializer().Serialize(new { version = 1, reservation = entry }));
        if (bytes.Length > Limit) throw new InvalidDataException("Control journal exceeds its bound.");
        using (var file = new FileStream(temporary,FileMode.Create,FileAccess.Write,FileShare.None,4096,FileOptions.WriteThrough))
        { file.Write(bytes,0,bytes.Length); file.Flush(true); }
        Check(temporary);
        if (File.Exists(path)) File.Replace(temporary,path,null);
        else File.Move(temporary,path);
        Check(path);
    }
    internal Dictionary<string, object> Pending() { return Locked(Read); }
    internal void Recover(Func<Dictionary<string,object>,bool> restore)
    {
        if (restore == null) throw new ArgumentNullException("restore");
        Locked(delegate {
            var entry=Read();
            if (entry == null) return true;
            if (!restore(entry)) throw new InvalidOperationException("Native recovery was not verified; intent retained.");
            Write(null);
            return true;
        });
    }
    // Hold the SAME cross-process mutex as Reserve through stop/swap/remove.
    // This is a conservative refusal path, not automatic firmware recovery.
    internal void Maintenance(Action action)
    {
        if (action == null) throw new ArgumentNullException("action");
        Locked(delegate {
            if (Read() != null) throw new InvalidOperationException("Unresolved cooling channel: broker maintenance refused; preserve service and recovery journal.");
            action();
            return true;
        });
    }
    internal string Reserve(NativeControlIdentity identity)
    {
        if (identity == null || !IdentityText(identity.HardwareId) || !IdentityText(identity.SensorId)) throw new ArgumentException("Invalid native control identity.");
        return Locked(delegate {
            if (Read() != null) throw new InvalidOperationException("An unresolved native control intent exists; do not resume or acquire another channel.");
            string id = Guid.NewGuid().ToString("N");
            var entry = new Dictionary<string, object> { {"id",id}, {"backend","lhm_0_9_6"}, {"hardware_id",identity.HardwareId}, {"sensor_id",identity.SensorId}, {"pump",identity.Pump} };
            if (identity.Recovery != null) { ValidateRecovery(identity.Recovery); entry.Add("recovery",identity.Recovery); }
            Write(entry);
            return id;
        });
    }
    internal void Complete(string id)
    {
        Locked(delegate {
            var entry = Read();
            if (entry == null || !String.Equals((string)entry["id"],id,StringComparison.Ordinal)) throw new InvalidOperationException("Control-journal reservation identity changed.");
            Write(null);
            return true;
        });
    }
    public void Dispose() { mutex.Dispose(); }
}
