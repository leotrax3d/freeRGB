// Inactive private-session candidate. Not reachable from production CLI/RPC.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Web.Script.Serialization;

internal interface INativeControlPort
{
    int Floor { get; }
    bool Owned { get; }
    void Acquire();
    void SetDuty(float percent);
    void Release();
}
internal sealed class NativeControlSession
{
    private readonly object gate = new object();
    private readonly INativeControlPort port;
    private readonly Func<long> clock;
    private readonly string session = Nonce();
    private string ticket, state = "active", reason;
    private long issued, deadline, lastNow;
    private int sequence, duty = 100;
    private const int LeaseMs = 1000, TicketMs = 250;

    private static string Nonce()
    {
        byte[] bytes = new byte[16];
        using (var random = RandomNumberGenerator.Create()) random.GetBytes(bytes);
        return BitConverter.ToString(bytes).Replace("-", "").ToLowerInvariant();
    }
    private NativeControlSession(INativeControlPort port, Func<long> clock)
    {
        if (port == null || clock == null) throw new ArgumentNullException();
        this.port = port; this.clock = clock;
        long before = clock();
        if (before < 0 || before > Int64.MaxValue - LeaseMs) throw new InvalidOperationException("Invalid session clock.");
        port.Acquire();
        issued = lastNow = clock();
        if (issued < before || issued > Int64.MaxValue - LeaseMs)
        {
            // Acquisition requested only 100%; do not lose recovery ownership.
            state = "recovery_failed"; reason = "acquisition_clock_fault";
            ticket = Nonce(); deadline = issued;
            return;
        }
        deadline = issued + LeaseMs; ticket = Nonce();
    }
#if NATIVE_CONTROL_TESTS
    internal static NativeControlSession ForTest(INativeControlPort port, Func<long> clock) { return new NativeControlSession(port, clock); }
#else
    internal static NativeControlSession CreateProtected(JournaledNativeControl port)
    {
        BrokerService.CheckInstallation(true);
        var timer = Stopwatch.StartNew();
        return new NativeControlSession(port, delegate { return timer.ElapsedMilliseconds; });
    }
#endif
    private long Now()
    {
        long now = clock();
        if (now < lastNow || now > Int64.MaxValue - LeaseMs) { Latch("clock_fault"); return lastNow; }
        lastNow = now;
        return now;
    }
    private void Latch(string fault)
    {
        if (state != "active") return;
        state = "emergency"; reason = fault;
        try { port.SetDuty(100); duty = 100; }
        catch { state = "recovery_failed"; reason = fault + ":fallback_failed"; }
        // Keep the reservation. An authenticated supervisor must explicitly
        // resolve/restore this channel; a lost client cannot silently release it.
    }
    internal void Fault(string fault) { lock (gate) Latch(fault); }
    internal bool Poll()
    {
        lock (gate)
        {
            long now = Now();
            if (state == "active" && now >= deadline) Latch("lease_expired");
            return state == "active";
        }
    }
    internal Dictionary<string, object> View()
    {
        lock (gate) return new Dictionary<string, object> {
            {"version",1}, {"session",session}, {"ticket",ticket}, {"sequence",sequence},
            {"state",state}, {"reason",reason}, {"duty_percent",duty},
            {"ticket_ms",TicketMs}, {"lease_ms",LeaseMs}, {"owned",port.Owned},
            {"physical_write_verified",false}
        };
    }
    private static bool SameToken(object raw, string expected) { return raw is string && String.Equals((string)raw, expected, StringComparison.Ordinal); }
    internal Dictionary<string, object> Execute(string record)
    {
        lock (gate)
        {
            long now = Now();
            if (state == "active" && now >= deadline) Latch("lease_expired");
            if (state != "active") return View();
            try
            {
                if (record == null || System.Text.Encoding.UTF8.GetByteCount(record) > 2048) throw new ArgumentException();
                var request = new JavaScriptSerializer { MaxJsonLength = 2048, RecursionLimit = 8 }.DeserializeObject(record) as Dictionary<string, object>;
                if (request == null || !request.ContainsKey("session") || !request.ContainsKey("ticket") || !request.ContainsKey("sequence") || !request.ContainsKey("action") ||
                    !SameToken(request["session"],session) || !SameToken(request["ticket"],ticket) ||
                    !(request["sequence"] is int) || sequence == Int32.MaxValue || (int)request["sequence"] != sequence + 1 ||
                    !(request["action"] is string)) throw new ArgumentException();
                string action = (string)request["action"];
                bool close = action == "close";
                if ((!close && action != "set") || request.Count != (close ? 4 : 5) ||
                    (!close && (!request.ContainsKey("duty_percent") || !(request["duty_percent"] is int) || (int)request["duty_percent"] < port.Floor || (int)request["duty_percent"] > 100))) throw new ArgumentException();
                if (now - issued > TicketMs) { Latch("ticket_expired"); return View(); }
                // A valid but delayed request cannot extend a lease from its
                // receipt time. Only this server-issued ticket determines expiry.
                deadline = issued + LeaseMs;
                sequence++;
                duty = close ? 100 : (int)request["duty_percent"];
                try
                {
                    port.SetDuty(duty);
                    long after = Now();
                    if (state != "active") return View();
                    if (after >= deadline) { Latch("lease_expired_during_write"); return View(); }
                    if (close) { port.Release(); state = "closed"; }
                    else { issued = after; ticket = Nonce(); }
                }
                catch { Latch("native_write_failed"); }
                return View();
            }
            catch (Exception error)
            {
                if (error is OutOfMemoryException) throw;
                Latch("invalid_session_record");
                return View();
            }
        }
    }
}
