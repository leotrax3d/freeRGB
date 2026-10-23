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

// Support tiers (DECISIONS.md #14). Verified needs audited register readback;
// LhmSupported/Unrestricted trust LHM's software-mode setter and are guarded by
// behaviour (RPM plausibility, telemetry, fail-safe return) instead.
internal enum NativeTier { Verified, LhmSupported, Unrestricted }

internal static class NativeTiers
{
    internal static string Name(NativeTier tier)
    { return tier == NativeTier.Verified ? "verified" : tier == NativeTier.LhmSupported ? "lhm_supported" : "unrestricted"; }
    internal static NativeTier Parse(object raw)
    {
        if (Object.Equals(raw,"verified")) return NativeTier.Verified;
        if (Object.Equals(raw,"lhm_supported")) return NativeTier.LhmSupported;
        if (Object.Equals(raw,"unrestricted")) return NativeTier.Unrestricted;
        throw new System.IO.InvalidDataException("Unknown native support tier.");
    }
}
// Tier/role/floor rules shared by the protected authorization, broker and worker.
// Legacy records (no tier) keep the verified-only 40% fan / 100% pump rules.
internal static class NativeAuthorizationRules
{
    internal static void Validate(IDictionary<string,object> value, int minimum, out NativeTier tier, out bool pump, out bool pumpUnlocked)
    {
        tier = value.ContainsKey("tier") ? NativeTiers.Parse(value["tier"]) : NativeTier.Verified;
        if (value.ContainsKey("tier") && tier == NativeTier.Verified) throw new InvalidDataException("Verified authorizations use the legacy record.");
        object role = value.ContainsKey("role") ? value["role"] : null;
        if (role != null && !Object.Equals(role,"fan") && !Object.Equals(role,"pump")) throw new InvalidDataException("Invalid protected channel role.");
        pump = Object.Equals(role,"pump");
        pumpUnlocked = value.ContainsKey("pump_unlocked") && Object.Equals(value["pump_unlocked"],true);
        if (value.ContainsKey("pump_unlocked") && (!pumpUnlocked || !pump || tier != NativeTier.Unrestricted))
            throw new InvalidDataException("Pump reduction requires an unrestricted, separately confirmed pump.");
        if (minimum < (tier == NativeTier.Unrestricted ? 0 : 40) || minimum > 100)
            throw new InvalidDataException("Invalid protected duty floor for this tier.");
        if (minimum < 100 && !(Object.Equals(role,"fan") || pumpUnlocked))
            throw new InvalidDataException("Reduced commissioning duty requires explicit protected fan authorization.");
    }
}
// Small protected state files next to the journal (SYSTEM writers only).
// Boot-bound lists expire with the Windows boot they were written in.
// Temperature sources for fan curves: SuperIO, CPU or GPU sensors. A list of
// up to three ("a,b,c") means "the hottest of them" (mixed curve source).
internal static class NativeSources
{
    private static readonly System.Text.RegularExpressions.Regex Control = new System.Text.RegularExpressions.Regex("^/lpc/[a-z0-9]+/[0-9]+/(control|fan)/[0-9]+$");
    private static readonly System.Text.RegularExpressions.Regex Temperature = new System.Text.RegularExpressions.Regex("^/(lpc/[a-z0-9]+|amdcpu|intelcpu|gpu-[a-z]+(-[a-z]+)*)/[0-9]+/temperature/[0-9]+$");
    internal static bool IsChannel(string id) { return id != null && id.Length <= 128 && Control.IsMatch(id); }
    internal static string[] Temperatures(string list)
    {
        if (list == null || list.Length > 400) throw new InvalidDataException("Invalid temperature sources.");
        var parts = list.Split(',');
        var seen = new HashSet<string>();
        if (parts.Length < 1 || parts.Length > 3) throw new InvalidDataException("One to three temperature sources required.");
        foreach (string part in parts)
            if (part.Length > 128 || !Temperature.IsMatch(part) || !seen.Add(part)) throw new InvalidDataException("Invalid temperature source identity.");
        return parts;
    }
    internal static bool IsTemperatureList(string list)
    {
        try { Temperatures(list); return true; } catch (InvalidDataException) { return false; }
    }
}
internal static class NativeStateFile
{
    [System.Runtime.InteropServices.DllImport("kernel32.dll")]
    private static extern ulong GetTickCount64();
    internal static DateTime BootUtc { get { return DateTime.UtcNow - TimeSpan.FromMilliseconds(GetTickCount64()); } }
    internal static HashSet<string> Load(string path, bool bootBound)
    {
        var items = new HashSet<string>();
        try
        {
            if (!File.Exists(path) || new FileInfo(path).Length > 16384) return items;
            var value = new JavaScriptSerializer { MaxJsonLength = 16384, RecursionLimit = 4 }.DeserializeObject(File.ReadAllText(path)) as Dictionary<string,object>;
            if (value == null || !Object.Equals(value["version"],1) || !(value["items"] is object[])) return items;
            DateTime boot;
            if (bootBound && (!(value["boot_utc"] is string) || !DateTime.TryParse((string)value["boot_utc"],null,System.Globalization.DateTimeStyles.RoundtripKind,out boot) ||
                Math.Abs((boot.ToUniversalTime() - BootUtc).TotalMinutes) > 2)) return items;
            foreach (object item in (object[])value["items"]) if (item is string && ((string)item).Length <= 1024) items.Add((string)item);
        }
        catch (Exception error) { if (error is OutOfMemoryException) throw; }
        return items;
    }
    internal static void Save(string path, IEnumerable<string> items)
    {
        string temporary = path + ".tmp";
        File.WriteAllText(temporary, new JavaScriptSerializer().Serialize(new { version = 1, boot_utc = BootUtc.ToString("o"), items = new List<string>(items) }));
        if (File.Exists(path)) File.Replace(temporary, path, null); else File.Move(temporary, path);
    }
}
internal sealed class NativeControlIdentity
{
    internal readonly string HardwareId, SensorId;
    internal readonly bool Pump;
    internal readonly Dictionary<string,object> Recovery;
    // Null for the verified tier (unchanged journal shape); otherwise the tier name.
    internal readonly string Tier;
    internal NativeControlIdentity(string hardware, string sensor, bool pump, Dictionary<string,object> recovery = null, string tier = null)
    { HardwareId = hardware; SensorId = sensor; Pump = pump; Recovery = recovery; Tier = tier; }
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
    internal static NativeControlJournal CreateForService()
    {
        BrokerService.CheckInstallation(false);
        return new NativeControlJournal(Path.GetDirectoryName(BrokerTransport.DirectoryPath),false);
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
    // Version 1 holds at most one reservation (unchanged legacy shape). Version 2
    // holds several, one per channel; it is written only while two or more are
    // pending, so single-channel and empty states stay readable by older releases.
    private const int MaxReservations = 16;
    private List<Dictionary<string, object>> ReadAll()
    {
        Check(path);
        if (Directory.Exists(path)) throw new InvalidDataException("Control journal path is not a file.");
        var entries = new List<Dictionary<string, object>>();
        if (!File.Exists(path)) return entries;
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
        if (document != null && document.Count == 2 && document.ContainsKey("version") && Object.Equals(document["version"],1) && document.ContainsKey("reservation"))
        {
            if (document["reservation"] != null) entries.Add(Entry(document["reservation"]));
            return entries;
        }
        if (document == null || document.Count != 2 || !document.ContainsKey("version") || !Object.Equals(document["version"],2) ||
            !document.ContainsKey("reservations") || !(document["reservations"] is object[]))
            throw new InvalidDataException("Unsupported control journal; recovery required.");
        var list = (object[])document["reservations"];
        if (list.Length < 2 || list.Length > MaxReservations) throw new InvalidDataException("Invalid control journal reservation count; recovery required.");
        var seen = new HashSet<string>();
        foreach (object raw in list)
        {
            var entry = Entry(raw);
            if (!seen.Add((string)entry["id"]) || !seen.Add("channel|" + (string)entry["hardware_id"] + "|" + (string)entry["sensor_id"]))
                throw new InvalidDataException("Duplicate control reservation; recovery required.");
            entries.Add(entry);
        }
        return entries;
    }
    private Dictionary<string, object> Read()
    {
        var entries = ReadAll();
        return entries.Count == 0 ? null : entries[0];
    }
    private static Dictionary<string, object> Entry(object raw)
    {
        var entry = raw as Dictionary<string, object>;
        Guid identifier;
        if (entry == null || entry.Count != 5 + (entry.ContainsKey("recovery") ? 1 : 0) + (entry.ContainsKey("tier") ? 1 : 0) || !entry.ContainsKey("id") || !(entry["id"] is string) || !Guid.TryParseExact((string)entry["id"],"N",out identifier) ||
            !entry.ContainsKey("backend") || !Object.Equals(entry["backend"],LhmPin.Backend) ||
            !entry.ContainsKey("hardware_id") || !(entry["hardware_id"] is string) || !IdentityText((string)entry["hardware_id"]) ||
            !entry.ContainsKey("sensor_id") || !(entry["sensor_id"] is string) || !IdentityText((string)entry["sensor_id"]) ||
            !entry.ContainsKey("pump") || !(entry["pump"] is bool))
            throw new InvalidDataException("Invalid control reservation; recovery required.");
        if (entry.ContainsKey("tier") && !Object.Equals(entry["tier"],"lhm_supported") && !Object.Equals(entry["tier"],"unrestricted"))
            throw new InvalidDataException("Invalid control reservation tier; recovery required.");
        if (entry.ContainsKey("recovery")) ValidateRecovery(entry["recovery"] as Dictionary<string,object>, entry.ContainsKey("tier"));
        return entry;
    }
    // A reservation is live only while its recorded worker lifetime is running.
    // Entries without lifetime evidence (legacy/fixtures) count as unresolved.
    internal static bool OwnerAlive(Dictionary<string, object> entry, Func<string,bool> alive)
    {
        var recovery = entry.ContainsKey("recovery") ? entry["recovery"] as Dictionary<string,object> : null;
        if (recovery == null || !recovery.ContainsKey("worker_identity")) return false;
        return alive((string)recovery["worker_identity"]);
    }
    private static bool Lifetime(object value)
    { return value is string && System.Text.RegularExpressions.Regex.IsMatch((string)value,"^[1-9][0-9]{0,9}:[1-9][0-9]{0,18}$"); }
    private static void ValidateRecovery(Dictionary<string,object> value, bool tiered)
    {
        // Channels without register readback record only lifetimes: a dead worker's
        // channel is driven to 100% (no original mode is known), never to a low duty.
        if (value != null && value.ContainsKey("kind") && Object.Equals(value["kind"],"lhm_fallback_v1"))
        {
            if (!tiered || value.Count != 3 || !value.ContainsKey("service_identity") || !Lifetime(value["service_identity"]) ||
                !value.ContainsKey("worker_identity") || !Lifetime(value["worker_identity"]))
                throw new InvalidDataException("Invalid native fallback recovery metadata.");
            return;
        }
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
    private void Write(List<Dictionary<string, object>> entries)
    {
        Check(path);
        string temporary = path + ".tmp"; Check(temporary);
        if (entries.Count > MaxReservations) throw new InvalidOperationException("Too many native control reservations.");
        object document = entries.Count < 2 ? (object)new { version = 1, reservation = entries.Count == 0 ? null : entries[0] } : new { version = 2, reservations = entries };
        byte[] bytes = Encoding.UTF8.GetBytes(new JavaScriptSerializer().Serialize(document));
        if (bytes.Length > Limit) throw new InvalidDataException("Control journal exceeds its bound.");
        using (var file = new FileStream(temporary,FileMode.Create,FileAccess.Write,FileShare.None,4096,FileOptions.WriteThrough))
        { file.Write(bytes,0,bytes.Length); file.Flush(true); }
        Check(temporary);
        if (File.Exists(path)) File.Replace(temporary,path,null);
        else File.Move(temporary,path);
        Check(path);
    }
    internal Dictionary<string, object> Pending() { return Locked(Read); }
    internal List<Dictionary<string, object>> PendingAll() { return Locked(ReadAll); }
    // Each entry is restored and cleared on its own; failures keep their intent.
    // skip leaves entries alone (e.g. a still-running original worker).
    internal void Recover(Func<Dictionary<string,object>,bool> restore) { Recover(restore,null); }
    internal void Recover(Func<Dictionary<string,object>,bool> restore, Func<Dictionary<string,object>,bool> skip)
    {
        if (restore == null) throw new ArgumentNullException("restore");
        Locked(delegate {
            var entries=ReadAll();
            var kept=new List<Dictionary<string,object>>();
            Exception failure=null;
            bool unverified=false;
            foreach (var entry in entries)
            {
                if (skip != null && skip(entry)) { kept.Add(entry); continue; }
                bool restored=false;
                try { restored=restore(entry); }
                catch (Exception error) { if (error is OutOfMemoryException) throw; if (failure == null) failure=error; }
                if (!restored) { kept.Add(entry); unverified=true; }
            }
            if (kept.Count != entries.Count) Write(kept);
            if (failure != null) throw new InvalidOperationException("Native recovery failed; intent retained.",failure);
            if (unverified) throw new InvalidOperationException("Native recovery was not verified; intent retained.");
            return true;
        });
    }
    // Serializes protected configuration writes with journal operations without
    // requiring an empty journal (running channels may stay reserved).
    internal void Serialized(Action action)
    {
        if (action == null) throw new ArgumentNullException("action");
        Locked(delegate { action(); return true; });
    }
    // Hold the SAME cross-process mutex as Reserve through stop/swap/remove.
    // This is a conservative refusal path, not automatic firmware recovery.
    internal void Maintenance(Action action)
    {
        if (action == null) throw new ArgumentNullException("action");
        Locked(delegate {
            if (ReadAll().Count != 0) throw new InvalidOperationException("Unresolved cooling channel: broker maintenance refused; preserve service and recovery journal.");
            action();
            return true;
        });
    }
    internal string Reserve(NativeControlIdentity identity) { return Reserve(identity,null); }
    // Several channels may be reserved at once, each by its own live worker. The
    // same channel twice, or any entry without a live owner (an unresolved orphan),
    // refuses: recovery must run first.
    internal string Reserve(NativeControlIdentity identity, Func<string,bool> alive)
    {
        if (identity == null || !IdentityText(identity.HardwareId) || !IdentityText(identity.SensorId)) throw new ArgumentException("Invalid native control identity.");
        return Locked(delegate {
            var entries = ReadAll();
            foreach (var existing in entries)
            {
                if ((string)existing["hardware_id"] == identity.HardwareId && (string)existing["sensor_id"] == identity.SensorId)
                    throw new InvalidOperationException("This native channel already has an unresolved intent; do not resume it.");
                if (alive == null || !OwnerAlive(existing,alive))
                    throw new InvalidOperationException("An unresolved native control intent exists; do not resume or acquire another channel.");
            }
            if (entries.Count >= MaxReservations) throw new InvalidOperationException("Too many native control reservations.");
            string id = Guid.NewGuid().ToString("N");
            var entry = new Dictionary<string, object> { {"id",id}, {"backend",LhmPin.Backend}, {"hardware_id",identity.HardwareId}, {"sensor_id",identity.SensorId}, {"pump",identity.Pump} };
            if (identity.Tier != null && identity.Tier != "lhm_supported" && identity.Tier != "unrestricted") throw new ArgumentException("Invalid native tier.");
            if (identity.Recovery != null) { ValidateRecovery(identity.Recovery, identity.Tier != null); entry.Add("recovery",identity.Recovery); }
            if (identity.Tier != null) entry.Add("tier",identity.Tier);
            entries.Add(entry);
            Write(entries);
            return id;
        });
    }
    // Releases may complete in any order; only the exact transaction is removed.
    internal void Complete(string id)
    {
        Locked(delegate {
            var entries = ReadAll();
            int index = entries.FindIndex(delegate(Dictionary<string,object> e) { return String.Equals((string)e["id"],id,StringComparison.Ordinal); });
            if (index < 0) throw new InvalidOperationException("Control-journal reservation identity changed.");
            entries.RemoveAt(index);
            Write(entries);
            return true;
        });
    }
    public void Dispose() { mutex.Dispose(); }
}
