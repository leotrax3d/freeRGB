// Read-only service; elevated installation lives in the one-shot deployment entry.
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.IO.Pipes;
using System.Reflection;
using System.Security.AccessControl;
using System.Security.Principal;
using System.ServiceProcess;
using System.Text;
using System.Threading;
using Microsoft.Win32;
using TimeoutException = System.TimeoutException;

internal sealed class BrokerService : ServiceBase
{
    private readonly ManualResetEvent stop = new ManualResetEvent(false);
    private readonly BrokerCache cache = new BrokerCache();
    private readonly object gate = new object();
    private NamedPipeServerStream pipe;
    private Process worker;
    private Thread readerThread, pipeThread;
    internal const string Name = "freeRGBMainboard";
    internal static readonly string RuntimePath = Path.Combine(BrokerTransport.DirectoryPath, "library");

    internal static bool IsSystem()
    {
        using (WindowsIdentity identity = WindowsIdentity.GetCurrent()) return identity.User.IsWellKnown(WellKnownSidType.LocalSystemSid);
    }

    internal static void CheckProtectedPath(string path)
    {
        if ((File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0) throw new InvalidOperationException("Reparse points are not permitted in the broker installation.");
        bool directory = Directory.Exists(path);
        FileSystemSecurity acl = directory ? (FileSystemSecurity)Directory.GetAccessControl(path) : File.GetAccessControl(path);
        var system = new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null);
        var admins = new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null);
        var installer = (SecurityIdentifier)new NTAccount(@"NT SERVICE\TrustedInstaller").Translate(typeof(SecurityIdentifier));
        var owner = (SecurityIdentifier)acl.GetOwner(typeof(SecurityIdentifier));
        if (!owner.Equals(system) && !owner.Equals(admins) && !owner.Equals(installer)) throw new InvalidOperationException("Untrusted broker file owner.");
        const FileSystemRights writes = FileSystemRights.Write | FileSystemRights.Delete | FileSystemRights.DeleteSubdirectoriesAndFiles | FileSystemRights.ChangePermissions | FileSystemRights.TakeOwnership;
        foreach (FileSystemAccessRule rule in acl.GetAccessRules(true, true, typeof(SecurityIdentifier)))
        {
            if (rule.AccessControlType != AccessControlType.Allow || (rule.PropagationFlags & PropagationFlags.InheritOnly) != 0 || (rule.FileSystemRights & writes) == 0) continue;
            if (!rule.IdentityReference.Equals(system) && !rule.IdentityReference.Equals(admins) && !rule.IdentityReference.Equals(installer)) throw new InvalidOperationException("Broker installation permits unprivileged modification.");
        }
    }

    internal static void CheckInstallation(bool child)
    {
        string expected = child ? Path.Combine(BrokerTransport.DirectoryPath, "freergb-mainboard.exe") : BrokerTransport.ServicePath;
        if (!IsSystem() || !String.Equals(Path.GetFullPath(Assembly.GetExecutingAssembly().Location), expected, StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Broker requires LocalSystem and its fixed protected installation path.");
        string current = BrokerTransport.DirectoryPath;
        string programFiles = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles);
        while (true)
        {
            CheckProtectedPath(current);
            if (String.Equals(current, programFiles, StringComparison.OrdinalIgnoreCase)) break;
            current = Path.GetDirectoryName(current);
            if (current == null) throw new InvalidOperationException("Invalid broker path.");
        }
        CheckTree(BrokerTransport.DirectoryPath);
        if (!File.Exists(Path.Combine(RuntimePath, "LibreHardwareMonitorLib.dll"))) throw new FileNotFoundException("Protected LHM runtime is missing.");
    }
    private static void CheckTree(string directory)
    {
        CheckProtectedPath(directory);
        foreach (string file in Directory.GetFiles(directory)) CheckProtectedPath(file);
        foreach (string child in Directory.GetDirectories(directory)) CheckTree(child);
    }

    private static void BeforeStart()
    {
#if BROKER_FIXTURE_FAIL_START
        throw new InvalidOperationException("Intentional SCM acceptance failure; must restore the previous version.");
#endif
    }
    protected override void OnStart(string[] args)
    {
        BeforeStart();
        CheckInstallation(false);
        // Owner comes only from the administrator-protected service registry key.
        SecurityIdentifier user;
        using (RegistryKey key = Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Services\" + Name + @"\Parameters"))
        {
            string sid = key == null ? null : key.GetValue("AllowedUserSid") as string;
            if (sid == null || (!sid.StartsWith("S-1-5-21-", StringComparison.Ordinal) && !sid.StartsWith("S-1-12-1-", StringComparison.Ordinal))) throw new InvalidOperationException("Broker owner SID is missing.");
            user = new SecurityIdentifier(sid);
        }
        // Claim the name before starting hardware. A pre-existing pipe fails closed.
        pipe = BrokerTransport.CreateServer(BrokerTransport.PipeName, user);
        readerThread = new Thread(ReadHardware) { IsBackground = true, Name = "freeRGB motherboard reader supervisor" };
        pipeThread = new Thread(Serve) { IsBackground = true, Name = "freeRGB motherboard snapshot server" };
        readerThread.Start(); pipeThread.Start();
    }

    private void ReadHardware()
    {
        while (!stop.WaitOne(0))
        {
            try
            {
                var start = new ProcessStartInfo(Path.Combine(BrokerTransport.DirectoryPath, "freergb-mainboard.exe"), "--direct-protected-library") {
                    UseShellExecute = false, CreateNoWindow = true, RedirectStandardInput = true, RedirectStandardOutput = true,
                    WorkingDirectory = Environment.GetFolderPath(Environment.SpecialFolder.System), StandardOutputEncoding = new UTF8Encoding(false, true)
                };
                using (var child = new Process { StartInfo = start })
                {
                    lock (gate) { if (stop.WaitOne(0)) return; child.Start(); worker = child; }
                    try
                    {
                        bool first = true;
                        while (!stop.WaitOne(0))
                        {
                            long began = BrokerCache.Now;
                            child.StandardInput.WriteLine("snapshot"); child.StandardInput.Flush();
                            cache.Store(BrokerTransport.ReadReply(child.StandardOutput.BaseStream, first ? 10000 : 3000), began);
#if BROKER_FIXTURE_CLIENT_AUTH
                            if (first)
                            {
                                using (var proof = BrokerTransport.Connect("freergb-native-auth-fixture-" + child.Id))
                                {
                                    var owner = (SecurityIdentifier)proof.GetAccessControl().GetOwner(typeof(SecurityIdentifier));
                                    if (!owner.IsWellKnown(WellKnownSidType.LocalSystemSid) || !BrokerTransport.IsExpectedWorker(proof,(uint)child.Id))
                                        throw new InvalidOperationException("Synthetic worker identity failed.");
                                    BrokerTransport.Write(proof,Encoding.ASCII.GetBytes("snapshot\n"));
                                    if (BrokerTransport.ReadReply(proof,2000) != "authenticated") throw new InvalidOperationException("Registered service client authentication failed.");
                                    BrokerTransport.Write(proof,new byte[] { 6 });
                                }
                            }
#endif
                            first = false;
                            if (stop.WaitOne(1000)) return;
                        }
                    }
                    finally
                    {
                        lock (gate) { if (!child.HasExited) child.Kill(); child.WaitForExit(2000); worker = null; }
                    }
                }
            }
            catch (Exception error)
            {
                if (error is OutOfMemoryException) throw;
                cache.Store(BrokerCache.Failure("Isolated motherboard reader failed (" + error.GetType().Name + ")."), BrokerCache.Now);
            }
            if (stop.WaitOne(5000)) return; // Bound crash/restart frequency.
        }
    }

    private void Serve()
    {
        try
        {
            while (!stop.WaitOne(0))
            {
                NamedPipeServerStream connection;
                lock (gate) { connection = pipe; }
                try
                {
                    connection.WaitForConnection();
                    if (BrokerTransport.ReadSnapshotRequest(connection))
                    {
                        byte[] response = Encoding.UTF8.GetBytes(cache.Read() + "\n");
                        BrokerTransport.Write(connection, response);
                        if (!BrokerTransport.ReadAcknowledgment(connection)) throw new IOException("Missing reply acknowledgment.");
                    }
                }
                catch (IOException) { }
                catch (TimeoutException) { }
                catch (System.AggregateException) { }
                // Retain the FIRST_PIPE_INSTANCE handle across all clients; otherwise a
                // disconnect/recreate gap would allow pipe-name takeover or false collisions.
                lock (gate)
                {
                    if (stop.WaitOne(0)) return;
                    if (connection.IsConnected) connection.Disconnect();
                }
            }
        }
        catch (Exception error)
        {
            if (error is OutOfMemoryException) throw;
            // Lost pipe ownership or unexpected transport failure: stop, never serve under a weaker ACL.
            Stop();
        }
    }

    protected override void OnStop()
    {
        stop.Set();
        lock (gate)
        {
            if (pipe != null) pipe.Dispose();
            try { if (worker != null && !worker.HasExited) worker.Kill(); }
            catch (InvalidOperationException) { }
        }
        if (readerThread != null && readerThread != Thread.CurrentThread) readerThread.Join(5000);
        if (pipeThread != null && pipeThread != Thread.CurrentThread) pipeThread.Join(3000);
    }

    private static int Main(string[] args)
    {
#if BROKER_INSTALLER
        if (args.Length != 0) return BrokerDeployment.Dispatch(args);
#else
        if (args.Length != 0) return 2;
#endif
        using (var service = new BrokerService { ServiceName = Name, CanStop = true, AutoLog = true }) ServiceBase.Run(service);
        return 0;
    }
}
