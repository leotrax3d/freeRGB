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
    private readonly NamedPipeServerStream pipe;
    private readonly Thread thread;
    private volatile bool stopping;
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
        if (value == null || value.Count != 7 || !Object.Equals(value["version"],1) ||
            !Object.Equals(value["floor_percent"],100) || !(value["critical_c"] is int) ||
            (int)value["critical_c"] < 40 || (int)value["critical_c"] > 100)
            throw new InvalidDataException("Only protected fixed100 commissioning authorization is accepted.");
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
            try
            {
                pipe.WaitForConnection();
                if (stopping) return;
                if (BrokerTransport.ReadRecord(pipe,2000,256) != "open") throw new InvalidDataException("Expected native session open.");
                var authorization=Authorization();
                stage="worker_start";
                string controlId=(string)authorization["control_id"];
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
                                while (!stopping)
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
}
