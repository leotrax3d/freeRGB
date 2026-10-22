// Inactive service-side peer for the private control-worker session protocol.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.IO.Pipes;
using System.Security.Principal;
using System.Text;
using System.Web.Script.Serialization;

internal sealed class NativeSessionReceipt
{
    internal string State, Reason;
    internal int DutyPercent;
    internal bool Owned;
}
internal sealed class NativeSessionClient : IDisposable
{
    private readonly object gate = new object();
    private readonly NamedPipeClientStream pipe;
    private readonly Func<long> clock;
    private Dictionary<string, object> reply;
    private long received;
    private bool terminal, disposed;
    private NativeSessionClient(NamedPipeClientStream pipe, Func<long> clock)
    {
        this.pipe = pipe; this.clock = clock;
        reply = Read(null,null);
        if ((string)reply["state"] != "active" || (int)reply["sequence"] != 0 || (int)reply["duty_percent"] != 100)
            throw new InvalidDataException("Invalid initial native session state.");
        received = clock();
    }
#if NATIVE_CONTROL_TESTS
    internal static NativeSessionClient ForTest(NamedPipeClientStream pipe, Func<long> clock)
    {
        try { return new NativeSessionClient(pipe,clock); } catch { pipe.Dispose(); throw; }
    }
#else
    internal static NativeSessionClient CreateProtected(NamedPipeClientStream pipe, uint expectedWorkerPid)
    {
        BrokerService.CheckInstallation(false);
        if (pipe == null || !pipe.IsConnected) throw new InvalidOperationException("Connected private worker pipe required.");
        var owner = (SecurityIdentifier)pipe.GetAccessControl().GetOwner(typeof(SecurityIdentifier));
        if (!owner.IsWellKnown(WellKnownSidType.LocalSystemSid) || !BrokerTransport.IsExpectedWorker(pipe,expectedWorkerPid))
            throw new InvalidOperationException("Native session peer is not the expected protected SYSTEM worker.");
        var timer = Stopwatch.StartNew();
        try { return new NativeSessionClient(pipe,delegate { return timer.ElapsedMilliseconds; }); }
        catch { pipe.Dispose(); throw; }
    }
#endif
    private static bool Token(object raw)
    {
        var value = raw as string;
        if (value == null || value.Length != 32) return false;
        foreach (char c in value) if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))) return false;
        return true;
    }
    private Dictionary<string, object> Read(Dictionary<string, object> prior, int? expected)
    {
        var value = new JavaScriptSerializer { MaxJsonLength = 2048, RecursionLimit = 8 }
            .DeserializeObject(BrokerTransport.ReadRecord(pipe,2000,2048)) as Dictionary<string,object>;
        if (value == null || !value.ContainsKey("version") || !Object.Equals(value["version"],1) ||
            !value.ContainsKey("session") || !Token(value["session"]) || !value.ContainsKey("ticket") || !Token(value["ticket"]) ||
            !value.ContainsKey("sequence") || !(value["sequence"] is int) || (int)value["sequence"] < 0 ||
            !value.ContainsKey("state") || !(value["state"] is string) ||
            !value.ContainsKey("reason") || (value["reason"] != null && (!(value["reason"] is string) || ((string)value["reason"]).Length > 128)) ||
            !value.ContainsKey("duty_percent") || !(value["duty_percent"] is int) || (int)value["duty_percent"] < (Object.Equals(value.ContainsKey("tier") ? value["tier"] : null,"unrestricted") ? 0 : 40) || (int)value["duty_percent"] > 100 ||
            !value.ContainsKey("owned") || !(value["owned"] is bool) ||
            !value.ContainsKey("physical_write_verified") || !Object.Equals(value["physical_write_verified"],false) ||
            !value.ContainsKey("ticket_ms") || !Object.Equals(value["ticket_ms"],250) || !value.ContainsKey("lease_ms") || !Object.Equals(value["lease_ms"],1000))
            throw new InvalidDataException("Invalid native session response.");
        string state = (string)value["state"];
        if (state != "active" && state != "closed" && state != "emergency" && state != "recovery_failed") throw new InvalidDataException("Unknown native session state.");
        if ((state == "active" && (!(bool)value["owned"] || value["reason"] != null)) || (state == "closed" && ((bool)value["owned"] || (int)value["duty_percent"] != 100)))
            throw new InvalidDataException("Inconsistent native ownership response.");
        if (prior != null)
        {
            if (!Object.Equals(value["session"],prior["session"])) throw new InvalidDataException("Native session identity changed.");
            int counter = (int)value["sequence"];
            if ((state == "active" || state == "closed") ? counter != expected.Value : (counter < (int)prior["sequence"] || counter > expected.Value))
                throw new InvalidDataException("Native response counter mismatch.");
            if (state == "active" && Object.Equals(value["ticket"],prior["ticket"])) throw new InvalidDataException("Native ticket was reused.");
        }
        return value;
    }
    private NativeSessionReceipt Send(int? duty)
    {
        lock (gate)
        {
            if (disposed || terminal) throw new InvalidOperationException("Native session is no longer active.");
            try
            {
                long now = clock();
                if (now < received || now - received > 200 || (int)reply["sequence"] == Int32.MaxValue)
                    throw new InvalidOperationException("Local native ticket window expired; do not replay or reconnect automatically.");
                int next = (int)reply["sequence"] + 1;
                var request = new Dictionary<string,object> { {"session",reply["session"]}, {"ticket",reply["ticket"]}, {"sequence",next}, {"action",duty.HasValue ? "set" : "close"} };
                if (duty.HasValue) request.Add("duty_percent",duty.Value);
                BrokerTransport.Write(pipe,Encoding.UTF8.GetBytes(new JavaScriptSerializer().Serialize(request)+"\n"));
                var result = Read(reply,next);
                string state = (string)result["state"];
                if ((state == "active" && (!duty.HasValue || (int)result["duty_percent"] != duty.Value)) || (state == "closed" && duty.HasValue))
                    throw new InvalidDataException("Native response does not match the requested operation.");
                reply = result; received = clock(); terminal = state != "active";
                return new NativeSessionReceipt { State=state, Reason=result["reason"] as string, DutyPercent=(int)result["duty_percent"], Owned=(bool)result["owned"] };
            }
            catch { Dispose(); throw; }
        }
    }
    internal NativeSessionReceipt SetDuty(int duty)
    {
        if (duty < (Object.Equals(reply.ContainsKey("tier") ? reply["tier"] : null,"unrestricted") ? 0 : 40) || duty > 100) throw new ArgumentOutOfRangeException("duty");
        return Send(duty);
    }
    internal NativeSessionReceipt Close() { return Send(null); }
    public void Dispose() { lock (gate) { if (disposed) return; disposed = true; pipe.Dispose(); } }
}
