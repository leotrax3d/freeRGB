// Service transport only. No curve/calibration policy; commissioning is fixed100.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.IO.Pipes;
using System.Security.Principal;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

internal sealed class NativeControlBroker : IDisposable
{
    internal const string PipeName="freergb-mainboard-control-v1";
    // Set by the service: an interactive session first stops the background
    // curve on the same channel (explicit return), then resumes it afterwards.
    internal static Action<string> SuspendBackground = null, ResumeBackground = null;
    private readonly NamedPipeServerStream pipe;
    private readonly Thread thread;
    private volatile bool stopping;
    private volatile bool maintenance;
    private readonly object admission=new object();
    private readonly ManualResetEvent idle=new ManualResetEvent(true);
    internal void PrepareMaintenance()
    {
        lock (admission) maintenance=true;
        if (!idle.WaitOne(20000)) throw new TimeoutException("Active native return did not finish; keep binaries and recovery state.");
    }
    internal NativeControlBroker(SecurityIdentifier owner)
    {
        pipe=BrokerTransport.CreateServer(PipeName,owner);
        thread=new Thread(Serve) { IsBackground=true,Name="freeRGB native session bridge" };
        thread.Start();
    }
    private static Dictionary<string,object> Authorization()
    {
        BrokerService.CheckInstallation(false);
        string path=Path.Combine(Path.GetDirectoryName(BrokerTransport.DirectoryPath),"mainboard-control-authorization.json");
        BrokerService.CheckProtectedPath(path);
        if (new FileInfo(path).Length > 4096) throw new InvalidDataException("Control authorization too large.");
        var value=new JavaScriptSerializer { MaxJsonLength=4096,RecursionLimit=8 }.DeserializeObject(File.ReadAllText(path)) as Dictionary<string,object>;
        int optional = value == null ? 0 : (value.ContainsKey("role") ? 1 : 0) + (value.ContainsKey("tier") ? 1 : 0) + (value.ContainsKey("pump_unlocked") ? 1 : 0);
        if (value == null || value.Count != 7 + optional || !Object.Equals(value["version"],1) ||
            !(value["floor_percent"] is int) || !(value["critical_c"] is int) ||
            (int)value["critical_c"] < 40 || (int)value["critical_c"] > 100)
            throw new InvalidDataException("Invalid protected commissioning authorization.");
        int minimum=(int)value["floor_percent"];
        NativeTier tier; bool pump, pumpUnlocked;
        NativeAuthorizationRules.Validate(value,minimum,out tier,out pump,out pumpUnlocked);
        DateTime expiry;
        if (!(value["expires_utc"] is string) || !DateTime.TryParseExact((string)value["expires_utc"],"o",System.Globalization.CultureInfo.InvariantCulture,
            System.Globalization.DateTimeStyles.RoundtripKind,out expiry) || expiry.Kind != DateTimeKind.Utc || expiry <= DateTime.UtcNow || expiry > DateTime.UtcNow.AddMinutes(5))
            throw new InvalidDataException("Commissioning authorization expired or invalid.");
        foreach (string key in new[] { "control_id","temperature_id","rpm_id" })
            if (!(value[key] is string) || !System.Text.RegularExpressions.Regex.IsMatch((string)value[key],"^/lpc/[a-z0-9]+/[0-9]+/(control|temperature|fan)/[0-9]+$"))
                throw new InvalidDataException("Invalid authorized native source identity.");
        return value;
    }
    private void Serve()
    {
        while (!stopping)
        {
            string stage="admission";
            string suspended=null;
            try
            {
                pipe.WaitForConnection();
                if (stopping) return;
                if (BrokerTransport.ReadRecord(pipe,2000,256) != "open") throw new InvalidDataException("Expected native session open.");
                lock (admission)
                {
                    if (maintenance || stopping) throw new InvalidOperationException("Native control admission closed for maintenance.");
                    idle.Reset();
                }
                var authorization=Authorization();
                stage="worker_start";
                string controlId=(string)authorization["control_id"];
                if (SuspendBackground != null) SuspendBackground(controlId);
                suspended=controlId;
                authorization.Remove("control_id");
                authorization.Remove("expires_utc");
                string binding=Convert.ToBase64String(Encoding.UTF8.GetBytes(new JavaScriptSerializer().Serialize(authorization)));
                string name="freergb-control-worker-"+Guid.NewGuid().ToString("N");
                var start=new ProcessStartInfo(Path.Combine(BrokerTransport.DirectoryPath,"freergb-mainboard.exe"),
                    "--control-protected-session "+name+" "+controlId+" "+binding) {
                    UseShellExecute=false,CreateNoWindow=true,RedirectStandardOutput=true,RedirectStandardError=true,
                    WorkingDirectory=Environment.GetFolderPath(Environment.SpecialFolder.System)
                };
                using (var worker=new Process { StartInfo=start })
                {
                    worker.Start();
                    try
                    {
                        try {
                            if (BrokerTransport.ReadRecord(worker.StandardOutput.BaseStream,10000,64).TrimEnd('\r') != "ready") throw new InvalidDataException("Native worker not ready.");
                        } catch {
                            if (worker.WaitForExit(100)) {
                                string detail=worker.StandardError.ReadToEnd();
                                File.WriteAllText(Path.Combine(Path.GetDirectoryName(BrokerTransport.DirectoryPath),"mainboard-worker-last-error.txt"),detail.Substring(0,Math.Min(2048,detail.Length)));
                            }
                            throw;
                        }
                        stage="worker_authentication";
                        using (var native=BrokerTransport.Connect(name))
                        {
                            var owner=(SecurityIdentifier)native.GetAccessControl().GetOwner(typeof(SecurityIdentifier));
                            if (!owner.IsWellKnown(WellKnownSidType.LocalSystemSid) || !BrokerTransport.IsExpectedWorker(native,(uint)worker.Id))
                                throw new InvalidOperationException("Unexpected protected control worker.");
                            string reply=BrokerTransport.ReadRecord(native,2000,2048);
                            stage="session";
                            try
                            {
                                while (!stopping && !maintenance)
                                {
                                    BrokerTransport.Write(pipe,Encoding.UTF8.GetBytes(reply+"\n"));
                                    var receipt=new JavaScriptSerializer { MaxJsonLength=2048 }.DeserializeObject(reply) as Dictionary<string,object>;
                                    if (receipt == null || !Object.Equals(receipt["state"],"active")) break;
                                    string request=BrokerTransport.ReadRecord(pipe,2000,2048);
                                    BrokerTransport.Write(native,Encoding.UTF8.GetBytes(request+"\n"));
                                    reply=BrokerTransport.ReadRecord(native,2000,2048);
                                }
                            }
                            finally
                            {
                                // Best-effort explicit return while this authenticated ticket
                                // remains valid. The native session independently rejects expiry.
                                var last=new JavaScriptSerializer { MaxJsonLength=2048 }.DeserializeObject(reply) as Dictionary<string,object>;
                                if (last != null && Object.Equals(last["state"],"active"))
                                {
                                    var close=new { session=last["session"],ticket=last["ticket"],sequence=(int)last["sequence"]+1,action="close" };
                                    BrokerTransport.Write(native,Encoding.UTF8.GetBytes(new JavaScriptSerializer().Serialize(close)+"\n"));
                                    BrokerTransport.ReadRecord(native,2000,2048);
                                }
                            }
                        }
                    }
                    finally
                    {
                        if (!worker.WaitForExit(2000)) { worker.Kill(); worker.WaitForExit(2000); }
                    }
                }
            }
            catch (Exception error)
            {
                if (error is OutOfMemoryException) throw;
                // No admission authorization or transport failure: no fake receipt.
                try {
                    string path=Path.Combine(Path.GetDirectoryName(BrokerTransport.DirectoryPath),"mainboard-control-last-error.json");
                    File.WriteAllText(path,new JavaScriptSerializer().Serialize(new {version=1,stage=stage,kind=error.GetType().Name}));
                } catch (IOException) { } catch (UnauthorizedAccessException) { }
            }
            finally
            {
                if (suspended != null && ResumeBackground != null) ResumeBackground(suspended);
                idle.Set();
                try { if (pipe.IsConnected) pipe.Disconnect(); }
                catch (ObjectDisposedException) { }
                catch (IOException) { }
            }
        }
    }
    public void Dispose()
    {
        stopping=true;
        pipe.Dispose();
        if (Thread.CurrentThread != thread && !thread.Join(18000))
            throw new InvalidOperationException("Native bridge still running; maintenance must remain blocked.");
    }

    internal static int Relay()
    {
        using (var connection=BrokerTransport.Connect(PipeName))
        {
            var owner=(SecurityIdentifier)connection.GetAccessControl().GetOwner(typeof(SecurityIdentifier));
            if (!owner.IsWellKnown(WellKnownSidType.LocalSystemSid) || !BrokerTransport.IsServiceServer(connection))
                throw new InvalidOperationException("Control endpoint is not the registered SYSTEM service.");
            BrokerService.CheckProtectedPath(BrokerTransport.ServicePath);
            BrokerTransport.Write(connection,Encoding.ASCII.GetBytes("open\n"));
            Stream input=Console.OpenStandardInput();
            bool first=true;
            while (true)
            {
                string reply=BrokerTransport.ReadRecord(connection,12000,2048);
                if (first) { Console.WriteLine("ready"); Console.Out.Flush(); first=false; }
                Console.WriteLine(reply); Console.Out.Flush();
                var result=new JavaScriptSerializer { MaxJsonLength=2048 }.DeserializeObject(reply) as Dictionary<string,object>;
                if (result == null || !Object.Equals(result["state"],"active")) return 0;
                string request=BrokerTransport.ReadRecord(input,2000,2048);
                BrokerTransport.Write(connection,Encoding.UTF8.GetBytes(request+"\n"));
            }
        }
    }
    // Hands a validated background configuration to the elevated, consented
    // service command (UAC). An empty channel list disables background control.
    internal static int RequestBackgroundConfiguration(string[] args)
    {
        if (args.Length != 2 || args[1].Length > 44000) return 2;
        try { BackgroundChannelConfig.ParseAll(Encoding.UTF8.GetString(Convert.FromBase64String(args[1]))); }
        catch (Exception error) { if (error is OutOfMemoryException) throw; return 2; }
        BrokerService.CheckProtectedPath(BrokerTransport.ServicePath);
        string sid;
        using (var user=WindowsIdentity.GetCurrent()) sid=user.User.Value;
        var start=new ProcessStartInfo(BrokerTransport.ServicePath,"--configure-cooling --consent "+sid+" "+args[1]) {
            UseShellExecute=true,Verb="runas",WindowStyle=ProcessWindowStyle.Hidden,
            WorkingDirectory=Environment.GetFolderPath(Environment.SpecialFolder.System)
        };
        using (var process=Process.Start(start))
        {
            if (!process.WaitForExit(120000)) throw new TimeoutException("Background configuration did not finish.");
            return process.ExitCode;
        }
    }
    internal static int RequestAuthorization(string[] args)
    {
        // Legacy: control temperature rpm critical minimum. Tiered appends: tier role.
        if (args.Length != 6 && args.Length != 8) return 2;
        foreach(int index in new[] {1,2,3})
            if (!System.Text.RegularExpressions.Regex.IsMatch(args[index],"^/lpc/[a-z0-9]+/[0-9]+/(control|temperature|fan)/[0-9]+$")) return 2;
        int critical,minimum;
        if (!Int32.TryParse(args[4],out critical) || critical < 40 || critical > 100 ||
            !Int32.TryParse(args[5],out minimum) || minimum < (args.Length == 8 ? 0 : 40) || minimum > 100) return 2;
        if (args.Length == 8 && ((args[6] != "lhm_supported" && args[6] != "unrestricted") || (args[7] != "fan" && args[7] != "pump"))) return 2;
        BrokerService.CheckProtectedPath(BrokerTransport.ServicePath);
        string sid;
        using (var user=WindowsIdentity.GetCurrent()) sid=user.User.Value;
        var start=new ProcessStartInfo(BrokerTransport.ServicePath,"--authorize-control --consent "+sid+" "+String.Join(" ",args,1,args.Length-1)) {
            UseShellExecute=true,Verb="runas",WindowStyle=ProcessWindowStyle.Hidden,
            WorkingDirectory=Environment.GetFolderPath(Environment.SpecialFolder.System)
        };
        using (var process=Process.Start(start))
        {
            if (!process.WaitForExit(120000)) throw new TimeoutException("Control authorization did not finish.");
            return process.ExitCode;
        }
    }
}
