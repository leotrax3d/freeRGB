// One-shot elevated maintenance. Never register a service from the per-user app/cache.
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using System.ServiceProcess;
using System.Text;
using System.Web.Script.Serialization;
using Microsoft.Win32;

internal interface IBrokerRegistration
{
    bool Exists { get; }
    bool Running { get; }
    void Stop();
    void Create();
    void Start();
    void Remove();
}

internal static class BrokerTransaction
{
    // Internal seam for disk/SCM failure acceptance; production callers supply fixed paths only.
    internal static void Apply(string target, string staged, string backup, IBrokerRegistration registration)
    {
        bool existed = registration.Exists, running = existed && registration.Running;
        bool saved = false, swapped = false, created = false, stopped = false;
        try
        {
            if (existed) { registration.Stop(); stopped = true; }
            if (Directory.Exists(target)) { Directory.Move(target, backup); saved = true; }
            Directory.Move(staged, target); swapped = true;
            if (!existed) { registration.Create(); created = true; }
            registration.Start();
        }
        catch (Exception failure)
        {
            try
            {
                if (swapped && registration.Exists) registration.Stop();
                if (created) registration.Remove();
                if (swapped) Directory.Move(target, staged);
                if (saved) Directory.Move(backup, target);
                if (running && stopped) registration.Start();
            }
            catch (Exception rollback) { throw new AggregateException("Broker update failed and rollback requires administrator repair. Protected backup retained: " + backup, failure, rollback); }
            throw;
        }
    }
}

internal sealed class BrokerRegistration : IBrokerRegistration
{
    internal const string KeyPath = @"SYSTEM\CurrentControlSet\Services\" + BrokerService.Name;
    private readonly string sid;
    internal BrokerRegistration(string sid) { this.sid = sid; }
    public bool Exists { get { using (var key = Registry.LocalMachine.OpenSubKey(KeyPath)) return key != null; } }
    public bool Running { get { using (var service = new ServiceController(BrokerService.Name)) return service.Status != ServiceControllerStatus.Stopped; } }
    internal void ValidateOwner()
    {
        using (var key = Registry.LocalMachine.OpenSubKey(KeyPath))
        {
            if (key == null) return;
            if (!Object.Equals(key.GetValue("ImagePath"), "\"" + BrokerTransport.ServicePath + "\"") ||
                !Object.Equals(key.GetValue("Type"), 16) || !Object.Equals(key.GetValue("ObjectName"), "LocalSystem"))
                throw new InvalidOperationException("A foreign or changed service registration uses the freeRGB name.");
        }
        using (var parameters = Registry.LocalMachine.OpenSubKey(KeyPath + @"\Parameters"))
            if (parameters == null || !Object.Equals(parameters.GetValue("AllowedUserSid"), sid))
                throw new InvalidOperationException("Broker belongs to another account or has no valid owner state.");
        BrokerDeployment.ValidateTree(BrokerTransport.DirectoryPath);
        BrokerDeployment.ValidateMarker(BrokerTransport.DirectoryPath, sid);
    }
    public void Stop()
    {
        if (!Exists) return;
        using (var service = new ServiceController(BrokerService.Name))
        {
            Process process = null;
            byte[] status = new byte[36]; int needed;
            if (!QueryServiceStatusEx(service.ServiceHandle, 0, status, status.Length, out needed)) throw new Win32Exception(Marshal.GetLastWin32Error());
            int pid = BitConverter.ToInt32(status, 28);
            if (pid > 0) { try { process = Process.GetProcessById(pid); } catch (ArgumentException) { } }
            using (process)
            {
            if (service.Status != ServiceControllerStatus.Stopped)
            {
                if (service.Status != ServiceControllerStatus.StopPending) service.Stop();
                service.WaitForStatus(ServiceControllerStatus.Stopped, TimeSpan.FromSeconds(15));
            }
            if (process != null && !process.WaitForExit(5000)) throw new System.TimeoutException("Stopped service process did not exit; update was not applied.");
            }
        }
    }
    public void Start()
    {
        using (var service = new ServiceController(BrokerService.Name))
        {
            service.Start();
            service.WaitForStatus(ServiceControllerStatus.Running, TimeSpan.FromSeconds(15));
        }
    }
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr OpenSCManager(string machine, string database, uint access);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateService(IntPtr manager, string name, string display, uint access,
        uint type, uint start, uint error, string image, string group, IntPtr tag, string dependencies, string account, string password);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr OpenService(IntPtr manager, string name, uint access);
    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern bool DeleteService(IntPtr service);
    [DllImport("advapi32.dll")]
    private static extern bool CloseServiceHandle(IntPtr handle);
    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern bool QueryServiceStatusEx(SafeHandle service, int level, byte[] status, int size, out int needed);

    public void Create()
    {
        IntPtr manager = OpenSCManager(null, null, 3);
        if (manager == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        try
        {
            // Win32 own process, automatic startup, normal error handling, LocalSystem.
            IntPtr service = CreateService(manager, BrokerService.Name, "freeRGB Mainboard Sensors", 0xF01FF,
                16, 2, 1, "\"" + BrokerTransport.ServicePath + "\"", null, IntPtr.Zero, null, null, null);
            if (service == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            CloseServiceHandle(service);
            try
            {
                var acl = new RegistrySecurity();
                acl.SetAccessRuleProtection(true, false);
                var admins = new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null);
                acl.SetOwner(admins);
                acl.AddAccessRule(new RegistryAccessRule(admins, RegistryRights.FullControl, InheritanceFlags.ContainerInherit, PropagationFlags.None, AccessControlType.Allow));
                acl.AddAccessRule(new RegistryAccessRule(new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null), RegistryRights.FullControl, InheritanceFlags.ContainerInherit, PropagationFlags.None, AccessControlType.Allow));
                acl.AddAccessRule(new RegistryAccessRule(new SecurityIdentifier(WellKnownSidType.BuiltinUsersSid, null), RegistryRights.ReadKey, InheritanceFlags.ContainerInherit, PropagationFlags.None, AccessControlType.Allow));
                using (var key = Registry.LocalMachine.CreateSubKey(KeyPath + @"\Parameters", RegistryKeyPermissionCheck.ReadWriteSubTree, RegistryOptions.None, acl))
                    key.SetValue("AllowedUserSid", sid, RegistryValueKind.String);
            }
            catch { Remove(); throw; }
        }
        finally { CloseServiceHandle(manager); }
    }
    public void Remove()
    {
        IntPtr manager = OpenSCManager(null, null, 1);
        if (manager == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        try
        {
            IntPtr service = OpenService(manager, BrokerService.Name, 0x10000);
            if (service == IntPtr.Zero)
            {
                int error = Marshal.GetLastWin32Error();
                if (error == 1060) return;
                throw new Win32Exception(error);
            }
            try { if (!DeleteService(service)) throw new Win32Exception(Marshal.GetLastWin32Error()); }
            finally { CloseServiceHandle(service); }
        }
        finally { CloseServiceHandle(manager); }
    }
}

internal static class BrokerDeployment
{
    internal static Dictionary<string, object> Manifest()
    {
        using (Stream stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("MainboardPackages"))
        using (var reader = new StreamReader(stream))
            return (Dictionary<string, object>)new JavaScriptSerializer().DeserializeObject(reader.ReadToEnd());
    }
    internal static FileStream VerifiedFile(string path, long size, string hash)
    {
        // Share-read only prevents replacement/writes between verification and extraction/copy.
        var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        try
        {
            using (var sha = SHA256.Create())
            {
                if (stream.Length != size || !String.Equals(BitConverter.ToString(sha.ComputeHash(stream)).Replace("-", ""), hash, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidDataException("Deployment payload hash/size mismatch: " + Path.GetFileName(path));
            }
            stream.Position = 0;
            return stream;
        }
        catch { stream.Dispose(); throw; }
    }
    internal static SecurityIdentifier Owner(string text)
    {
        // Local/domain accounts and cloud accounts; broad well-known groups are not owners.
        var sid = new SecurityIdentifier(text);
        if ((!text.StartsWith("S-1-5-21-", StringComparison.Ordinal) && !text.StartsWith("S-1-12-1-", StringComparison.Ordinal)) || text.Length > 184)
            throw new ArgumentException("An explicit installing-user account SID is required.");
        return sid;
    }
    internal static void ValidateTree(string directory)
    {
        BrokerService.CheckProtectedPath(directory);
        foreach (string file in Directory.GetFiles(directory)) BrokerService.CheckProtectedPath(file);
        foreach (string child in Directory.GetDirectories(directory)) ValidateTree(child);
    }
    internal static void ValidateMarker(string root, string sid)
    {
        string path = Path.Combine(root, "installation.json");
        if (new FileInfo(path).Length > 4096) throw new InvalidDataException("Invalid broker installation marker.");
        var marker = new JavaScriptSerializer().Deserialize<Dictionary<string, object>>(File.ReadAllText(path));
        if (!marker.ContainsKey("version") || !Object.Equals(marker["version"], 1) || !marker.ContainsKey("owner_sid") || !Object.Equals(marker["owner_sid"], sid))
            throw new InvalidDataException("Broker installation marker does not match its owner.");
    }
    private static void ProtectedDirectory(string path)
    {
        if (Directory.Exists(path)) { BrokerService.CheckProtectedPath(path); return; }
        var acl = new DirectorySecurity();
        acl.SetAccessRuleProtection(true, false);
        var admins = new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null);
        acl.SetOwner(admins);
        foreach (var sid in new[] { admins, new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null) })
            acl.AddAccessRule(new FileSystemAccessRule(sid, FileSystemRights.FullControl, InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit, PropagationFlags.None, AccessControlType.Allow));
        acl.AddAccessRule(new FileSystemAccessRule(new SecurityIdentifier(WellKnownSidType.BuiltinUsersSid, null), FileSystemRights.ReadAndExecute, InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit, PropagationFlags.None, AccessControlType.Allow));
        Directory.CreateDirectory(path, acl);
        BrokerService.CheckProtectedPath(path);
    }
    private static void Copy(Stream source, string destination)
    {
        using (var output = new FileStream(destination, FileMode.CreateNew, FileAccess.Write, FileShare.None)) source.CopyTo(output);
    }
    private static void SetNewFileOwners(string directory)
    {
        foreach (string file in Directory.GetFiles(directory))
        {
            var acl = File.GetAccessControl(file);
            acl.SetOwner(new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null));
            File.SetAccessControl(file, acl);
        }
        foreach (string child in Directory.GetDirectories(directory))
        {
            var acl = Directory.GetAccessControl(child);
            acl.SetOwner(new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null));
            Directory.SetAccessControl(child, acl);
            SetNewFileOwners(child);
        }
    }
    internal static void StageRuntime(string archive, string notices, string destination)
    {
        var manifest = Manifest();
        var packages = (Dictionary<string, object>)manifest["packages"];
        var lhm = (Dictionary<string, object>)packages["lhm"];
        Directory.CreateDirectory(destination);
        using (FileStream input = VerifiedFile(archive, Convert.ToInt64(lhm["size"]), (string)lhm["sha256"]))
        using (var zip = new ZipArchive(input, ZipArchiveMode.Read))
        {
            foreach (var entry in zip.Entries)
            {
                string name = entry.FullName;
                if (!System.Text.RegularExpressions.Regex.IsMatch(name, @"^[A-Za-z0-9_.-]+\.dll$") ||
                    System.Text.RegularExpressions.Regex.IsMatch(name, @"^(Aga\.Controls|OxyPlot.*|Microsoft\.Win32\.TaskScheduler)\.dll$")) continue;
                if (entry.Length > 30 * 1024 * 1024) throw new InvalidDataException("Unexpected runtime entry size.");
                using (var source = entry.Open()) Copy(source, Path.Combine(destination, name));
            }
        }
        if (AssemblyName.GetAssemblyName(Path.Combine(destination, "LibreHardwareMonitorLib.dll")).Version.ToString() != "0.9.6.0")
            throw new InvalidDataException("Unexpected runtime assembly version.");
        foreach (Dictionary<string, object> notice in (object[])manifest["notices"])
        {
            string name = (string)notice["file"];
            using (var source = VerifiedFile(Path.Combine(notices, name), Convert.ToInt64(notice["size"]), (string)notice["sha256"])) Copy(source, Path.Combine(destination, name));
        }
        File.WriteAllText(Path.Combine(destination, "SOURCES.txt"), "Unmodified LHM: https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/v0.9.6\r\nEmbedded modules: https://github.com/namazso/PawnIO.Modules/releases/tag/0.1.6\r\n");
    }

    private static void Install(string sid, string selfHash, string archive, string notices)
    {
        string parent = Path.GetDirectoryName(BrokerTransport.DirectoryPath);
        BrokerService.CheckProtectedPath(Path.GetDirectoryName(parent));
        ProtectedDirectory(parent);
        var registration = new BrokerRegistration(sid);
        registration.ValidateOwner();
        if (!registration.Exists && Directory.Exists(BrokerTransport.DirectoryPath))
        {
            ValidateTree(BrokerTransport.DirectoryPath);
            ValidateMarker(BrokerTransport.DirectoryPath, sid); // Refuse unrelated leftover directories.
        }
        string staged = Path.Combine(parent, "MainboardBroker-stage-" + Guid.NewGuid().ToString("N"));
        string backup = Path.Combine(parent, "MainboardBroker-backup-" + Guid.NewGuid().ToString("N"));
        ProtectedDirectory(staged);
        try
        {
            string source = Assembly.GetExecutingAssembly().Location;
            // Hash captured by the bootstrap before UAC, then held immutable through copying.
            using (var input = VerifiedFile(source, new FileInfo(source).Length, selfHash)) Copy(input, Path.Combine(staged, "freergb-mainboard-service.exe"));
            using (var input = VerifiedFile(Path.Combine(Path.GetDirectoryName(source), "freergb-mainboard.exe"), BrokerBuild.HelperSize, BrokerBuild.HelperSha256))
                Copy(input, Path.Combine(staged, "freergb-mainboard.exe"));
            StageRuntime(archive, notices, Path.Combine(staged, "library"));
            File.WriteAllText(Path.Combine(staged, "installation.json"), new JavaScriptSerializer().Serialize(new {
                version = 1, owner_sid = sid, protocol_version = 1, helper_sha256 = BrokerBuild.HelperSha256
            }));
            SetNewFileOwners(staged); ValidateTree(staged);
            BrokerTransaction.Apply(BrokerTransport.DirectoryPath, staged, backup, registration);
            if (Directory.Exists(backup))
            {
                try { DeleteProtectedTree(backup); }
                catch (IOException) { Console.WriteLine("Update succeeded; protected previous-version backup retained: " + backup); }
                catch (UnauthorizedAccessException) { Console.WriteLine("Update succeeded; protected previous-version backup retained: " + backup); }
            }
        }
        finally
        {
            // Never remove the backup on failure: it may be the only recoverable old build.
            if (Directory.Exists(staged)) DeleteProtectedTree(staged);
        }
    }
    private static void DeleteProtectedTree(string path)
    {
        // Only internally generated descendants of our fixed protected parent can be removed.
        string parent = Path.GetDirectoryName(BrokerTransport.DirectoryPath) + Path.DirectorySeparatorChar;
        string full = Path.GetFullPath(path);
        if (!full.StartsWith(parent, StringComparison.OrdinalIgnoreCase) || full.Length <= parent.Length)
            throw new InvalidOperationException("Cleanup target escaped the protected broker parent.");
        ValidateTree(full);
        Directory.Delete(full, true);
    }
    private static void Remove(string sid)
    {
        var registration = new BrokerRegistration(sid);
        if (!registration.Exists)
        {
            if (!Directory.Exists(BrokerTransport.DirectoryPath)) return;
            ValidateTree(BrokerTransport.DirectoryPath); ValidateMarker(BrokerTransport.DirectoryPath, sid);
        }
        else { registration.ValidateOwner(); registration.Stop(); registration.Remove(); }
        if (Directory.Exists(BrokerTransport.DirectoryPath)) DeleteProtectedTree(BrokerTransport.DirectoryPath);
    }
    internal static int Dispatch(string[] args)
    {
        bool install = args.Length == 6 && args[0] == "--install" && args[1] == "--consent";
        bool remove = args.Length == 3 && args[0] == "--remove" && args[1] == "--consent";
        if (!install && !remove) return 2;
        try
        {
            string sid = Owner(args[2]).Value;
            if (install && !System.Text.RegularExpressions.Regex.IsMatch(args[3], "^[a-fA-F0-9]{64}$")) throw new ArgumentException("Pre-elevation service hash is required.");
            using (var identity = WindowsIdentity.GetCurrent())
                if (!new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator)) throw new UnauthorizedAccessException("Explicit elevated maintenance is required.");
            // A single administrator-owned protected lock serializes installs/removals across sessions.
            string parent = Path.GetDirectoryName(BrokerTransport.DirectoryPath);
            BrokerService.CheckProtectedPath(Path.GetDirectoryName(parent)); ProtectedDirectory(parent);
            string lockPath = Path.Combine(parent, "mainboard-maintenance.lock");
            bool existingLock = File.Exists(lockPath);
            if (existingLock) BrokerService.CheckProtectedPath(lockPath);
            using (var maintenance = new FileStream(lockPath, FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None))
            {
                if (!existingLock)
                {
                    var acl = File.GetAccessControl(lockPath);
                    acl.SetOwner(new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null));
                    File.SetAccessControl(lockPath, acl);
                }
                BrokerService.CheckProtectedPath(maintenance.Name);
                if (install) Install(sid, args[3], args[4], args[5]); else Remove(sid);
            }
            Console.WriteLine(install ? "Protected mainboard broker installed and running. Fan writes remain disabled." : "Owned mainboard broker removed. Shared PawnIO and .NET are unchanged.");
            return 0;
        }
        catch (Exception error) { Console.Error.WriteLine(error.Message); return 1; }
    }
}
