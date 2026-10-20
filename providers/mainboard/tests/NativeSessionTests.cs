// Private-session tests. All native operations target an in-memory port only.
using System;
using System.Collections.Generic;
using System.IO.Pipes;
using System.Security.Principal;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;

internal sealed class SessionPort : INativeControlPort
{
    private readonly object gate = new object();
    private readonly List<int> writes = new List<int>();
    private bool owned, seenLow;
    internal readonly ManualResetEvent Fallback = new ManualResetEvent(false);
    internal bool FailHundred;
    internal Action<int> AfterWrite;
    public int Floor { get; set; }
    public bool Owned { get { lock (gate) return owned; } }
    internal int[] Writes { get { lock (gate) return writes.ToArray(); } }
    public void Acquire() { lock (gate) { if (owned) throw new InvalidOperationException(); owned = true; writes.Add(100); } }
    public void SetDuty(float percent)
    {
        lock (gate)
        {
            if (!owned || percent < Floor || percent > 100 || (FailHundred && percent == 100)) throw new InvalidOperationException();
            writes.Add((int)percent);
            if (percent < 100) seenLow = true;
            if (seenLow && percent == 100) Fallback.Set();
            if (AfterWrite != null) AfterWrite((int)percent);
        }
    }
    public void Release() { lock (gate) { writes.Add(-1); owned = false; } }
}
internal static class NativeSessionTests
{
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer();
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    internal static string Command(Dictionary<string, object> reply, string action, int? duty)
    {
        var value = new Dictionary<string, object> { {"session",reply["session"]}, {"ticket",reply["ticket"]}, {"sequence",(int)reply["sequence"]+1}, {"action",action} };
        if (duty.HasValue) value.Add("duty_percent",duty.Value);
        return Json.Serialize(value);
    }
    private static void Policy()
    {
        long now = 0;
        var port = new SessionPort { Floor = 40 };
        var session = NativeControlSession.ForTest(port, delegate { return now; });
        string first = Command(session.View(),"set",40);
        var reply = session.Execute(first);
        Assert((string)reply["state"] == "active" && port.Writes.Length == 2 && port.Writes[1] == 40,"Valid session command failed.");
        Assert((string)session.Execute(first)["state"] == "emergency", "Replayed command did not latch full speed.");
        Assert(port.Writes.Length == 3 && port.Writes[2] == 100 && port.Owned,"Replay fallback lost recovery ownership.");
        session.Execute(Command(reply,"set",60));
        Assert(port.Writes.Length == 3,"Terminal session accepted another write.");

        port = new SessionPort { Floor = 40 }; now = 0;
        session = NativeControlSession.ForTest(port, delegate { return now; });
        reply = session.View(); now = 251;
        Assert((string)session.Execute(Command(reply,"set",40))["reason"] == "ticket_expired", "Buffered command refreshed an expired ticket.");
        Assert(port.Writes.Length == 2 && port.Writes[1] == 100,"Expired ticket reached requested low duty.");

        port = new SessionPort { Floor = 40 }; now = 0;
        session = NativeControlSession.ForTest(port, delegate { return now; });
        now = 200; reply = session.Execute(Command(session.View(),"set",40));
        now = 999; Assert(session.Poll(),"Lease expired early.");
        now = 1000; Assert(!session.Poll() && port.Writes[2] == 100,"Lease was extended from receipt rather than ticket issue time.");

        foreach (string invalid in new[] { "{}", "[]", "{\"action\":\"set\"}", "{\"session\":\"wrong\"}" })
        {
            port = new SessionPort { Floor = 40 }; now = 0;
            session = NativeControlSession.ForTest(port, delegate { return now; });
            Assert((string)session.Execute(invalid)["state"] == "emergency","Malformed record accepted.");
        }
        for (int test = 0; test < 6; test++)
        {
            port = new SessionPort { Floor = 40 }; now = 0;
            session = NativeControlSession.ForTest(port, delegate { return now; });
            var request = (Dictionary<string,object>)Json.DeserializeObject(Command(session.View(),"set",40));
            switch (test)
            {
                case 0: request["session"] = "wrong"; break;
                case 1: request["ticket"] = "wrong"; break;
                case 2: request["sequence"] = 2; break;
                case 3: request["path"] = "not-a-native-parameter"; break;
                case 4: request["duty_percent"] = 40.5; break;
                default: request["action"] = "close"; break; // unexpected duty field
            }
            Assert((string)session.Execute(Json.Serialize(request))["state"] == "emergency" && port.Writes.Length == 2 && port.Writes[1] == 100,"Invalid capability, counter, shape or duty reached a native low write.");
        }
        port = new SessionPort { Floor = 100 }; now = 0;
        session = NativeControlSession.ForTest(port, delegate { return now; });
        Assert((string)session.Execute(Command(session.View(),"set",99))["state"] == "emergency","Pump reduction accepted.");
        Assert(Array.TrueForAll(port.Writes,delegate(int value) { return value == 100; }),"Pump received low output.");

        port = new SessionPort { Floor = 40 }; now = 0;
        session = NativeControlSession.ForTest(port, delegate { return now; });
        reply = session.Execute(Command(session.View(),"set",60));
        now = -1; session.Poll();
        Assert((string)session.View()["reason"] == "clock_fault" && port.Writes[2] == 100,"Reversed clock did not latch.");

        port = new SessionPort { Floor = 40 }; now = 0;
        session = NativeControlSession.ForTest(port, delegate { return now; });
        port.AfterWrite = delegate(int value) { if (value == 40) now = 1200; };
        reply = session.Execute(Command(session.View(),"set",40));
        Assert((string)reply["reason"] == "lease_expired_during_write" && port.Writes[2] == 100,"Slow completed write renewed an expired lease.");

        port = new SessionPort { Floor = 40 }; now = 0;
        session = NativeControlSession.ForTest(port, delegate { return now; });
        reply = session.Execute(Command(session.View(),"set",40));
        port.FailHundred = true; session.Fault("fixture_fault");
        Assert((string)session.View()["state"] == "recovery_failed" && port.Owned,"Failed fallback was hidden or released.");

        port = new SessionPort { Floor = 40 }; now = 0;
        session = NativeControlSession.ForTest(port, delegate { return now; });
        reply = session.Execute(Command(session.View(),"set",40));
        reply = session.Execute(Command(reply,"close",null));
        Assert((string)reply["state"] == "closed" && !port.Owned && port.Writes[2] == 100 && port.Writes[3] == -1,"Explicit close did not request full speed before firmware restoration.");
    }
    private static Dictionary<string, object> Read(NamedPipeClientStream pipe)
    {
        return (Dictionary<string,object>)Json.DeserializeObject(BrokerTransport.ReadRecord(pipe,3000,2048));
    }
    private static void Pipe(bool disconnect, bool oversized)
    {
        string name = "freergb-native-session-fixture-" + Guid.NewGuid().ToString("N");
        var user = WindowsIdentity.GetCurrent().User;
        var port = new SessionPort { Floor = 40 };
        var clock = System.Diagnostics.Stopwatch.StartNew();
        using (var server = BrokerTransport.CreateServer(name,user))
        {
            NativeControlSession session = null;
            var serving = Task.Run(delegate {
                server.WaitForConnection();
                session = NativeControlSession.ForTest(port,delegate { return clock.ElapsedMilliseconds; });
                NativeSessionTransport.RunForTest(server,session);
            });
            using (var client = BrokerTransport.Connect(name))
            {
                var hello = Read(client);
                Assert(!BrokerTransport.IsServiceClient(server),"Fixture client authenticated as the installed service.");
                if (oversized)
                {
                    BrokerTransport.Write(client,new byte[2048]);
                    Assert(serving.Wait(3000),"Oversized unterminated command blocked the session.");
                }
                else
                {
                    BrokerTransport.Write(client,Encoding.UTF8.GetBytes(Command(hello,"set",40)+"\n"));
                    Assert((string)Read(client)["state"] == "active","Pipe command rejected.");
                    if (disconnect) client.Dispose();
                    Assert(port.Fallback.WaitOne(1700),"Waiting for input suppressed the watchdog fallback.");
                    Assert(port.Writes[port.Writes.Length-1] == 100 && port.Owned,"Transport fault did not hold recovery ownership at full speed.");
                    if (!disconnect) Assert(!serving.IsCompleted,"Watchdog test did not act independently of the blocked input read.");
                }
            }
            Assert(serving.Wait(4000),"Native transport did not cancel/finish its pending read.");
            Assert((string)session.View()["state"] == "emergency" && port.Owned,"Transport silently restored/released after emergency.");
        }
    }
    internal static void Run()
    {
        Policy(); Pipe(false,false); Pipe(true,false); Pipe(false,true);
        Console.WriteLine("PASS: one-use session tickets, anti-replay/expiry, bounded real pipe input, independent input-stall watchdog, disconnect, close and failed recovery. Synthetic port only.");
    }
}
