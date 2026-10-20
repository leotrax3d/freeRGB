// Runs without service registration, elevation, LHM, drivers or hardware access.
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.IO.Pipes;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;

internal static class BrokerTests
{
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action, string message)
    {
        try { action(); } catch (ArgumentException) { return; } catch (InvalidOperationException) { return; } catch (InvalidDataException) { return; }
        throw new Exception(message);
    }
    private const string Sample = "{\"version\":1,\"status\":\"ready\",\"message\":\"fixture\",\"hardware\":[{\"id\":\"board\",\"name\":\"Fixture\",\"sensors\":[{\"id\":\"temp\",\"name\":\"Fixture\",\"kind\":\"temperature\",\"value\":42,\"age_ms\":500,\"control_available\":false}]}]}";
    private static Dictionary<string, object> Sensor(string reply)
    {
        var root = (Dictionary<string, object>)new JavaScriptSerializer().DeserializeObject(reply);
        var hardware = (Dictionary<string, object>)((object[])root["hardware"])[0];
        return (Dictionary<string, object>)((object[])hardware["sensors"])[0];
    }
    private static void CacheTests()
    {
        var cache = new BrokerCache();
        Assert(cache.Read().Contains("faulted"), "Uninitialized cache must not be ready.");
        cache.Store(Sample, BrokerCache.Now - Stopwatch.Frequency * 2);
        var sensor = Sensor(cache.Read());
        Assert(Convert.ToInt64(sensor["age_ms"]) >= 2500 && Object.Equals(sensor["value"], 42), "Relaying must preserve sample age.");
        cache.Store(Sample.Replace("500,", "4900,"), BrokerCache.Now - Stopwatch.Frequency);
        Assert(Sensor(cache.Read())["value"] == null, "Expired upstream sample must become unavailable.");
        cache.Store(Sample, BrokerCache.Now - Stopwatch.Frequency * 6);
        Assert(cache.Read().Contains("expired"), "Expired cache must fail closed.");
        Reject(delegate { cache.Store(Sample.Replace("500,", "-1,"), BrokerCache.Now); }, "Negative ages accepted.");
        Reject(delegate { cache.Store(Sample.Replace("500,", "0.5,"), BrokerCache.Now); }, "Fractional ages accepted.");
        Reject(delegate { cache.Store(Sample.Replace("\"version\":1", "\"version\":2"), BrokerCache.Now); }, "Unsupported protocol accepted.");
        Reject(delegate { cache.Store(Sample.Replace("ready", "arbitrary"), BrokerCache.Now); }, "Unknown status accepted.");
        Reject(delegate { cache.Store(Sample, BrokerCache.Now + Stopwatch.Frequency * 10); }, "Future timestamp accepted.");
        Reject(delegate { cache.Store(new String('x', BrokerTransport.Limit), BrokerCache.Now); }, "Oversize snapshot accepted.");
    }
    private static void RequestTests()
    {
        foreach (string bad in new[] { "set\n", "snapshot\r\n", "snapshotX", "", "load C:\\evil.dll\n" })
            using (var bytes = new MemoryStream(Encoding.UTF8.GetBytes(bad)))
                Assert(!BrokerTransport.ReadSnapshotRequest(bytes), "Unexpected broker command accepted.");
        using (var bytes = new MemoryStream(Encoding.UTF8.GetBytes("snapshot\n"))) Assert(BrokerTransport.ReadSnapshotRequest(bytes), "Snapshot command rejected.");
        using (var bytes = new MemoryStream(Encoding.UTF8.GetBytes("{}\nextra"))) Reject(delegate { BrokerTransport.ReadReply(bytes, 100); }, "Trailing reply bytes accepted.");
        using (var bytes = new MemoryStream(new byte[BrokerTransport.Limit])) Reject(delegate { BrokerTransport.ReadReply(bytes, 1000); }, "Unterminated oversize reply accepted.");
        using (var bytes = new MemoryStream(new byte[] { 0xff, 10 }))
        {
            bool rejected = false;
            try { BrokerTransport.ReadReply(bytes, 100); } catch (DecoderFallbackException) { rejected = true; }
            Assert(rejected, "Invalid UTF-8 accepted.");
        }
    }
    private static void PipeTests()
    {
        SecurityIdentifier user;
        using (var identity = WindowsIdentity.GetCurrent()) user = identity.User;
        string name = "freergb-broker-test-" + Guid.NewGuid().ToString("N");
        using (var server = BrokerTransport.CreateServer(name, user))
        {
            bool duplicateRejected = false;
            try { using (BrokerTransport.CreateServer(name, user)) { } } catch (Win32Exception) { duplicateRejected = true; }
            Assert(duplicateRejected, "Pre-existing pipe name not rejected.");
            bool found = false;
            foreach (PipeAccessRule rule in server.GetAccessControl().GetAccessRules(true, true, typeof(SecurityIdentifier)))
            {
                Assert(!rule.IdentityReference.Equals(new SecurityIdentifier(WellKnownSidType.WorldSid, null)), "Everyone access present.");
                Assert(!rule.IdentityReference.Equals(new SecurityIdentifier(WellKnownSidType.AnonymousSid, null)), "Anonymous access present.");
                if (rule.IdentityReference.Equals(user))
                {
                    found = true;
                    Assert(((uint)rule.PipeAccessRights & 4) == 0, "Client can create pipe instances.");
                    Assert(((uint)rule.PipeAccessRights & BrokerTransport.ClientRights) == BrokerTransport.ClientRights, "Required client rights missing.");
                }
            }
            Assert(found, "Client SID missing from pipe ACL.");
            Task serving = Task.Run(delegate
            {
                server.WaitForConnection();
                Assert(BrokerTransport.ReadSnapshotRequest(server), "Pipe request rejected.");
                server.RunAsClient(delegate
                {
                    using (var identity = WindowsIdentity.GetCurrent())
                        Assert(identity.ImpersonationLevel == TokenImpersonationLevel.Identification, "Client allowed a stronger impersonation token.");
                });
                byte[] response = Encoding.UTF8.GetBytes(Sample + "\n");
                server.Write(response, 0, response.Length);
            });
            using (var client = BrokerTransport.Connect(name))
            {
                Assert(!BrokerTransport.IsExpectedServer(client, BrokerTransport.ServicePath), "Spoofed production server trusted.");
                Assert(!BrokerTransport.IsServiceServer(client), "Fixture PID was trusted as a registered SYSTEM service.");
                Assert(BrokerTransport.IsExpectedServer(client, Process.GetCurrentProcess().MainModule.FileName), "Actual fixture identity not recognized.");
                byte[] request = Encoding.ASCII.GetBytes("snapshot\n"); client.Write(request, 0, request.Length);
                Assert(BrokerTransport.ReadReply(client, 2000) == Sample, "Pipe response altered.");
            }
            Assert(serving.Wait(3000), "Pipe server did not finish.");
        }
        using (var server = BrokerTransport.CreateServer(name, user))
        {
            Task serving = Task.Run(delegate
            {
                server.WaitForConnection();
                var clock = Stopwatch.StartNew(); bool timeout = false;
                try { BrokerTransport.ReadSnapshotRequest(server); } catch (TimeoutException) { timeout = true; }
                Assert(timeout && clock.ElapsedMilliseconds < 3500, "Silent client request not bounded.");
            });
            using (var client = BrokerTransport.Connect(name)) Assert(serving.Wait(4000), "Silent client test stalled.");
        }
        using (var server = BrokerTransport.CreateServer(name, user))
        {
            Task serving = Task.Run(delegate { server.WaitForConnection(); Thread.Sleep(500); });
            using (var client = BrokerTransport.Connect(name))
            {
                bool timeout = false;
                try { BrokerTransport.ReadReply(client, 100); } catch (TimeoutException) { timeout = true; }
                Assert(timeout, "Silent server reply not bounded.");
            }
            Assert(serving.Wait(2000), "Silent server test stalled.");
        }
        // Keep one server handle through successful and timed-out clients. No name gap.
        using (var server = BrokerTransport.CreateServer(name, user))
        {
            Task serving = Task.Run(delegate
            {
                for (int i = 0; i < 3; i++)
                {
                    server.WaitForConnection();
                    if (i == 1)
                    {
                        bool timeout = false;
                        try { BrokerTransport.ReadSnapshotRequest(server); } catch (TimeoutException) { timeout = true; }
                        Assert(timeout, "Silent middle client did not time out.");
                    }
                    else
                    {
                        Assert(BrokerTransport.ReadSnapshotRequest(server), "Reconnected request rejected.");
                        BrokerTransport.Write(server, Encoding.UTF8.GetBytes(Sample + "\n"));
                        Assert(BrokerTransport.ReadAcknowledgment(server), "Missing receipt acknowledgment.");
                    }
                    server.Disconnect();
                }
            });
            for (int i = 0; i < 3; i++)
            {
                NamedPipeClientStream connected = null;
                var clock = Stopwatch.StartNew();
                while (connected == null && clock.ElapsedMilliseconds < 4000)
                {
                    try { connected = BrokerTransport.Connect(name); }
                    catch (Win32Exception error) { if (error.NativeErrorCode != 231) throw; Thread.Sleep(10); }
                }
                Assert(connected != null, "Reconnection failed.");
                using (var client = connected)
                {
                    if (i == 1) Thread.Sleep(2300);
                    else
                    {
                        BrokerTransport.Write(client, Encoding.ASCII.GetBytes("snapshot\n"));
                        Assert(BrokerTransport.ReadReply(client, 2000) == Sample, "Reconnected response lost.");
                        BrokerTransport.Write(client, new byte[] { 6 });
                    }
                }
            }
            Assert(serving.Wait(4000), "Persistent server did not finish.");
        }
    }
    private static int Main()
    {
        CacheTests(); RequestTests(); PipeTests();
        Reject(delegate { BrokerService.CheckInstallation(true); }, "Unprotected worker startup accepted.");
        Reject(delegate { NativeControl.CreateProtected(new object(), false); }, "Unprotected native control factory accepted.");
        Reject(delegate { NativeControlSession.CreateProtected(null); }, "Unprotected native session factory accepted.");
        Reject(delegate { NativeSessionTransport.RunProtected(null, null); }, "Unprotected native transport accepted.");
        Reject(delegate { NativeControlJournal.CreateProtected(); }, "Unprotected native journal factory accepted.");
        Reject(delegate { JournaledNativeControl.CreateProtected(null,null); }, "Unprotected journaled port accepted.");
        Assert(typeof(NativeControl).GetMethod("ForTest", System.Reflection.BindingFlags.Static | System.Reflection.BindingFlags.NonPublic) == null, "Test-only native control factory leaked into production compilation.");
        Assert(typeof(NativeControlSession).GetMethod("ForTest", System.Reflection.BindingFlags.Static | System.Reflection.BindingFlags.NonPublic) == null && typeof(NativeSessionTransport).GetMethod("RunForTest", System.Reflection.BindingFlags.Static | System.Reflection.BindingFlags.NonPublic) == null,"Native session test entry leaked into production compilation.");
        Assert(typeof(NativeControlJournal).GetMethod("ForTest",System.Reflection.BindingFlags.Static | System.Reflection.BindingFlags.NonPublic) == null,"Test-directory journal factory leaked into production compilation.");
        string temporary = Path.GetTempFileName();
        try { Reject(delegate { BrokerService.CheckProtectedPath(temporary); }, "User-writable broker payload accepted."); }
        finally { File.Delete(temporary); }
        Console.WriteLine("PASS: broker ACL, identity, exclusive pipe ownership, bounded commands/replies, cache age and protected-worker guard. No hardware or service installation.");
        return 0;
    }
}
