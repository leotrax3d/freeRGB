// Candidate ordering bridge: durable intent before I/O, clear only after release.
using System;

internal sealed class JournaledNativeControl : INativeControlPort
{
    private readonly INativeControlPort port;
    private readonly NativeControlJournal journal;
    private readonly NativeControlIdentity identity;
    private readonly object gate = new object();
    private string reservation;
    private JournaledNativeControl(INativeControlPort port, NativeControlJournal journal, NativeControlIdentity identity)
    { this.port = port; this.journal = journal; this.identity = identity; }
#if NATIVE_CONTROL_TESTS
    internal static JournaledNativeControl ForTest(INativeControlPort port, NativeControlJournal journal, NativeControlIdentity identity)
    { return new JournaledNativeControl(port,journal,identity); }
#else
    internal static JournaledNativeControl CreateProtected(NativeControl port, NativeControlJournal journal)
    {
        BrokerService.CheckInstallation(true);
        return new JournaledNativeControl(port,journal,port.Identity());
    }
#endif
    public int Floor { get { return port.Floor; } }
    public bool Owned { get { lock (gate) return port.Owned; } }
    public void Acquire()
    {
        lock (gate)
        {
            if (reservation != null || port.Owned) throw new InvalidOperationException("Journaled native acquisition already pending.");
            reservation = journal.Reserve(identity); // Must finish/flush before ANY native I/O.
            port.Acquire();
        }
    }
    public void SetDuty(float percent) { lock (gate) { if (reservation == null) throw new InvalidOperationException("No native journal intent."); port.SetDuty(percent); } }
    public void Release()
    {
        lock (gate)
        {
            if (reservation == null) return;
            port.Release();
            if (port.Owned) throw new InvalidOperationException("Native release did not relinquish the channel.");
            journal.Complete(reservation); // A failed clear leaves the intent for retry/recovery.
            reservation = null;
        }
    }
}
