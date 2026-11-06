// Protected maintenance handshake. Admission closes before drain; the installer
// subsequently holds the existing native reservation mutex through replacement.
using System;
using System.Collections.Generic;
using System.IO;
using System.ServiceProcess;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

internal static class MainboardMaintenance
{
    private static string Root { get { return Path.GetDirectoryName(BrokerTransport.DirectoryPath); } }
    private static string PathFor(string name) { return Path.Combine(Root,name); }
    private static void Write(string name, object value)
    {
        string destination=PathFor(name), temporary=PathFor("maintenance-"+Guid.NewGuid().ToString("N")+".tmp");
        BrokerService.CheckProtectedPath(Root);
        if (File.Exists(destination)) BrokerService.CheckProtectedPath(destination);
        try {
            byte[] bytes=Encoding.UTF8.GetBytes(new JavaScriptSerializer().Serialize(value));
            using (var file=new FileStream(temporary,FileMode.CreateNew,FileAccess.Write,FileShare.None,4096,FileOptions.WriteThrough))
            { file.Write(bytes,0,bytes.Length); file.Flush(true); }
            var acl=File.GetAccessControl(temporary);
            acl.SetOwner(new System.Security.Principal.SecurityIdentifier(BrokerService.IsSystem() ?
                System.Security.Principal.WellKnownSidType.LocalSystemSid : System.Security.Principal.WellKnownSidType.BuiltinAdministratorsSid,null));
            File.SetAccessControl(temporary,acl);
            if (File.Exists(destination)) File.Replace(temporary,destination,null); else File.Move(temporary,destination);
        } finally { if(File.Exists(temporary))File.Delete(temporary); }
    }
    private static Dictionary<string,object> Read(string name)
    {
        string path=PathFor(name); BrokerService.CheckProtectedPath(path);
        if (new FileInfo(path).Length > 2048) throw new InvalidDataException("Maintenance record too large.");
        return new JavaScriptSerializer {MaxJsonLength=2048,RecursionLimit=8}.DeserializeObject(File.ReadAllText(path)) as Dictionary<string,object>;
    }
    internal static void Publish()
    {
        BrokerService.CheckInstallation(false);
        Write("mainboard-control-capabilities.json",new {version=1,maintenance=true,service_identity=BrokerTransport.CurrentProcessIdentity()});
    }
    // Maintenance drain order (owner acceptance D6, 2026-11-05): background curves
    // hold journal entries, so they return to firmware first; on a refused drain
    // they start again instead of staying off until the next service start.
    internal static bool Drain(Action stopBackground, Func<bool> service, Action restartBackground)
    {
        if (stopBackground != null) stopBackground();
        bool ready = false;
        try { ready = service(); }
        finally { if (!ready && restartBackground != null) restartBackground(); }
        return ready;
    }
    internal static bool Service(NativeControlBroker controls)
    {
#if NATIVE_CONTROL_TESTS
        throw new InvalidOperationException("No installed service in native unit tests.");
#else
        bool ready = false;
        BrokerService.CheckInstallation(false);
        var request=Read("mainboard-maintenance-request.json");
        DateTime expiry;
        if (request == null || request.Count != 3 || !Object.Equals(request["version"],1) || !(request["id"] is string) ||
            !System.Text.RegularExpressions.Regex.IsMatch((string)request["id"],"^[a-f0-9]{32}$") || !(request["expires_utc"] is string) ||
            !DateTime.TryParseExact((string)request["expires_utc"],"o",System.Globalization.CultureInfo.InvariantCulture,System.Globalization.DateTimeStyles.RoundtripKind,out expiry) ||
            expiry.Kind != DateTimeKind.Utc || expiry <= DateTime.UtcNow || expiry > DateTime.UtcNow.AddSeconds(40))
            throw new InvalidDataException("Invalid protected maintenance request.");
        File.Delete(PathFor("mainboard-maintenance-request.json"));
        string id=(string)request["id"];
        try {
            controls.PrepareMaintenance();
            using (var journal=NativeControlJournal.CreateForService())
            {
                var deadline=System.Diagnostics.Stopwatch.StartNew();
                while (journal.Pending() != null && deadline.ElapsedMilliseconds < 5000) Thread.Sleep(100);
                journal.Maintenance(delegate { Write("mainboard-maintenance-ready.json",new {version=1,id=id,ready=true}); ready = true; });
            }
        } catch {
            Write("mainboard-maintenance-ready.json",new {version=1,id=id,ready=false});
            // The failure receipt is authoritative; never assert return or reopen.
        }
        return ready;
#endif
    }
    internal static void RequestIfSupported()
    {
#if BROKER_INSTALLER
        if (!File.Exists(PathFor("mainboard-control-capabilities.json"))) return;
        using (var service=new ServiceController(BrokerService.Name))
        {
            if (service.Status != ServiceControllerStatus.Running) return;
            var capabilities=Read("mainboard-control-capabilities.json");
            if (capabilities == null || !Object.Equals(capabilities["version"],1) ||
                !Object.Equals(capabilities["service_identity"],BrokerTransport.ServiceIdentity())) return;
            if (!Object.Equals(capabilities["maintenance"],true)) throw new InvalidDataException("Unsupported maintenance capability.");
            string id=Guid.NewGuid().ToString("N");
            Write("mainboard-maintenance-request.json",new {version=1,id=id,expires_utc=DateTime.UtcNow.AddSeconds(35).ToString("o")});
            service.ExecuteCommand(129);
            var deadline=System.Diagnostics.Stopwatch.StartNew();
            while (deadline.ElapsedMilliseconds < 30000)
            {
                if (File.Exists(PathFor("mainboard-maintenance-ready.json")))
                {
                    var reply=Read("mainboard-maintenance-ready.json");
                    if (reply != null && Object.Equals(reply["version"],1) && Object.Equals(reply["id"],id))
                    {
                        if (!Object.Equals(reply["ready"],true)) throw new InvalidOperationException("Native return failed; maintenance refused.");
                        return;
                    }
                }
                Thread.Sleep(100);
            }
            throw new System.TimeoutException("Native maintenance acknowledgement missing; keep running service and binaries.");
        }
#endif
    }
    // Elevated, consented write of the protected background configuration,
    // followed by a reload request to the running service (command 130).
    internal static int ConfigureCooling(string[] args)
    {
#if BROKER_INSTALLER
        try {
            if (args.Length != 4 || args[0] != "--configure-cooling" || args[1] != "--consent") return 2;
            using (var identity=System.Security.Principal.WindowsIdentity.GetCurrent())
                if (!new System.Security.Principal.WindowsPrincipal(identity).IsInRole(System.Security.Principal.WindowsBuiltInRole.Administrator))
                    throw new UnauthorizedAccessException("Explicit elevated cooling configuration required.");
            if (!String.Equals(System.Reflection.Assembly.GetExecutingAssembly().Location,BrokerTransport.ServicePath,StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException("Use the installed protected configuration service.");
            new BrokerRegistration(args[2]).ValidateOwner();
            string text=Encoding.UTF8.GetString(Convert.FromBase64String(args[3]));
            BackgroundChannelConfig.ParseAll(text);
            var document=new JavaScriptSerializer { MaxJsonLength=32768,RecursionLimit=10 }.DeserializeObject(text);
            Write(BackgroundCooling.ConfigName,document);
            using (var service=new ServiceController(BrokerService.Name))
                if (service.Status == ServiceControllerStatus.Running) service.ExecuteCommand(130);
            return 0;
        } catch (Exception error) { Console.Error.WriteLine(error.GetType().Name); return 1; }
#else
        return 2;
#endif
    }
    internal static int AuthorizeControl(string[] args)
    {
#if BROKER_INSTALLER
        try {
            if ((args.Length != 8 && args.Length != 10) || args[0] != "--authorize-control" || args[1] != "--consent") return 2;
            using (var identity=System.Security.Principal.WindowsIdentity.GetCurrent())
                if (!new System.Security.Principal.WindowsPrincipal(identity).IsInRole(System.Security.Principal.WindowsBuiltInRole.Administrator))
                    throw new UnauthorizedAccessException("Explicit elevated control authorization required.");
            if (!String.Equals(System.Reflection.Assembly.GetExecutingAssembly().Location,BrokerTransport.ServicePath,StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException("Use the installed protected authorization service.");
            new BrokerRegistration(args[2]).ValidateOwner();
            if (!NativeSources.IsChannel(args[3]) || !NativeSources.IsTemperatureList(args[4]) || !NativeSources.IsChannel(args[5]))
                throw new ArgumentException("Invalid selected source identity.");
            int critical,minimum;
            if (!Int32.TryParse(args[6],out critical) || critical < 40 || critical > 100 ||
                !Int32.TryParse(args[7],out minimum) || minimum < (args.Length == 10 ? 0 : 40) || minimum > 100) throw new ArgumentException("Invalid native control limits.");
            object record;
            if (args.Length == 8)
            {
                // Verified/legacy record, identical to earlier releases.
                record = new {version=1,control_id=args[3],temperature_id=args[4],rpm_id=args[5],
                    role="fan",floor_percent=minimum,critical_c=critical,expires_utc=DateTime.UtcNow.AddMinutes(4).ToString("o")};
            }
            else
            {
                var tiered = new Dictionary<string,object> { {"version",1},{"control_id",args[3]},{"temperature_id",args[4]},{"rpm_id",args[5]},
                    {"role",args[9]},{"tier",args[8]},{"floor_percent",minimum},{"critical_c",critical},{"expires_utc",DateTime.UtcNow.AddMinutes(4).ToString("o")} };
                if (args[9] == "pump" && minimum < 100) tiered.Add("pump_unlocked",true);
                NativeTier tier; bool pump, pumpUnlocked;
                NativeAuthorizationRules.Validate(tiered,minimum,out tier,out pump,out pumpUnlocked);
                record = tiered;
            }
            // Merge with other channels' unexpired authorizations (one per channel),
            // so several channels can be authorized and run at the same time.
            var entry = record as Dictionary<string,object> ?? new JavaScriptSerializer().DeserializeObject(new JavaScriptSerializer().Serialize(record)) as Dictionary<string,object>;
            using (var journal=NativeControlJournal.CreateForMaintenance())
                journal.Serialized(delegate {
                    var list = new List<object> { entry };
                    string path = PathFor("mainboard-control-authorization.json");
                    if (File.Exists(path))
                    {
                        BrokerService.CheckProtectedPath(path);
                        try
                        {
                            var old = new JavaScriptSerializer { MaxJsonLength=16384,RecursionLimit=8 }.DeserializeObject(File.ReadAllText(path)) as Dictionary<string,object>;
                            var previous = old != null && Object.Equals(old["version"],2) && old["authorizations"] is object[] ? (object[])old["authorizations"] : new object[] { old };
                            foreach (object raw in previous)
                            {
                                var item = raw as Dictionary<string,object>;
                                if (item == null || Object.Equals(item["control_id"],entry["control_id"]) || list.Count >= 8) continue;
                                try { NativeControlBroker.ValidateAuthorization(item); list.Add(item); } catch (InvalidDataException) { } // expired or invalid
                            }
                        }
                        catch (ArgumentException) { } catch (InvalidOperationException) { }
                    }
                    Write("mainboard-control-authorization.json",new Dictionary<string,object> { {"version",2},{"authorizations",list} });
                });
            return 0;
        } catch (Exception error) { Console.Error.WriteLine(error.GetType().Name); return 1; }
#else
        return 2;
#endif
    }
}
