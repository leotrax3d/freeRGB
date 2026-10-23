// Version-pinned native adapter for protected workers only (fixed100 diagnostic,
// live sessions, background curves). Never called from the GUI/RPC loop.
using System;
using System.Reflection;
using System.IO;
using System.Runtime.CompilerServices;
using System.Collections.Generic;

internal interface INativeControlEvidence
{
    Dictionary<string,object> Recovery { get; }
    void VerifyDuty(float percent);
    void VerifyReturn();
}

// Missing register evidence for an otherwise valid LHM control. Every other
// evidence failure (vendor check, manual mode, ISA mutex) still refuses.
internal sealed class NativeEvidenceUnavailableException : InvalidOperationException
{
    internal NativeEvidenceUnavailableException(string message) : base(message) { }
}

internal sealed class NativeControl : INativeControlPort, INativeReturnEvidence
{
    private sealed class Reservation { internal readonly object Gate = new object(); internal NativeControl Owner; }
    private static readonly ConditionalWeakTable<object, Reservation> Reservations = new ConditionalWeakTable<object, Reservation>();
    private readonly object control;
    private readonly Reservation reservation;
    private readonly PropertyInfo mode, value;
    private readonly FieldInfo cachedMode;
    private readonly MethodInfo prime, software, firmware;
    private readonly int floor;
    private readonly bool pump;
    private readonly NativeTier requested;
    private bool owned;
    private INativeControlEvidence evidence;
    private Dictionary<string,object> fallback; // Lifetime-only recovery without readback.
    private bool returnVerified, returned;
    public bool FirmwareReturnVerified { get { lock (reservation.Gate) return returnVerified && !owned; } }
    // Owned SetDefault callback completed and LHM kept Default mode. Register proof
    // exists only when FirmwareReturnVerified is also true (verified tier).
    public bool FirmwareReturned { get { lock (reservation.Gate) return returned && !owned; } }
    public bool JournalCleared { get { return false; } } // Only the journal wrapper can assert this.

    private NativeControl(object control, bool pump, int? commissioningFloor = null, NativeTier tier = NativeTier.Verified, bool pumpUnlocked = false)
    {
        if (control == null) throw new ArgumentNullException("control");
        Type type = control.GetType();
        AssemblyName assembly = type.Assembly.GetName();
        if (type.FullName != "LibreHardwareMonitor.Hardware.Control" || assembly.Name != "LibreHardwareMonitorLib" || assembly.Version != LhmPin.Assembly)
            throw new InvalidOperationException("Native control requires the pinned LHM " + LhmPin.Version + " control implementation.");
        this.control = control;
        this.pump = pump;
        requested = tier;
        // Only an explicit unrestricted unlock lowers the 40% fan floor, and only a
        // second, separate pump confirmation lets a pump leave 100%.
        if (pumpUnlocked && tier != NativeTier.Unrestricted) throw new InvalidOperationException("Pump reduction requires an unrestricted unlock.");
        floor = pump && !pumpUnlocked ? 100 : (commissioningFloor ?? 40);
        if (floor < (tier == NativeTier.Unrestricted ? 0 : 40) || floor > 100) throw new InvalidOperationException("Unsupported commissioning floor.");
        mode = type.GetProperty("ControlMode");
        value = type.GetProperty("SoftwareValue");
        cachedMode = type.GetField("_mode", BindingFlags.Instance | BindingFlags.NonPublic);
        prime = value == null ? null : value.GetSetMethod(true);
        software = type.GetMethod("SetSoftware", new[] { typeof(float) });
        firmware = type.GetMethod("SetDefault", Type.EmptyTypes);
        if (mode == null || value == null || value.PropertyType != typeof(float) || cachedMode == null || cachedMode.FieldType != mode.PropertyType ||
            mode.PropertyType.FullName != "LibreHardwareMonitor.Hardware.ControlMode" || prime == null || !prime.IsPrivate || software == null || firmware == null ||
            Convert.ToInt32(Enum.Parse(mode.PropertyType, "Undefined")) != 0 || Convert.ToInt32(Enum.Parse(mode.PropertyType, "Software")) != 1 || Convert.ToInt32(Enum.Parse(mode.PropertyType, "Default")) != 2)
            throw new InvalidOperationException("Unsupported native control metadata; no writes permitted.");
        double minimum = Convert.ToDouble(type.GetProperty("MinSoftwareValue").GetValue(control, null));
        double maximum = Convert.ToDouble(type.GetProperty("MaxSoftwareValue").GetValue(control, null));
        if (Double.IsNaN(minimum) || Double.IsInfinity(minimum) || Double.IsNaN(maximum) || Double.IsInfinity(maximum) || minimum < 0 || minimum > floor || maximum != 100)
            throw new InvalidOperationException("Native control does not support the required safe duty range.");
        reservation = Reservations.GetValue(control, delegate(object key) { return new Reservation(); });
    }

#if NATIVE_CONTROL_TESTS
    // Only present in the separate synthetic-callback acceptance binary.
    internal static NativeControl ForTest(object control, bool pump) { return new NativeControl(control, pump); }
    internal static NativeControl ForTest(object control, bool pump, INativeControlEvidence evidence)
    { var adapter = new NativeControl(control,pump); adapter.evidence = evidence; return adapter; }
    internal static NativeControl ForTest(object control, bool pump, int floor, NativeTier tier, bool pumpUnlocked, INativeControlEvidence evidence, Dictionary<string,object> fallback = null)
    { var adapter = new NativeControl(control,pump,floor,tier,pumpUnlocked); adapter.evidence = evidence; adapter.fallback = fallback; return adapter; }
#else
    internal static NativeControl CreateProtected(object control, bool pump)
    { return CreateProtectedCore(control,pump,null); }
    internal static NativeControl CreateCommissioningProtected(object control, int minimum)
    {
        if (minimum < 40 || minimum > 100) throw new InvalidOperationException("Invalid commissioning floor.");
        return CreateProtectedCore(control,minimum == 100,minimum,NativeTier.Verified,false);
    }
    // Tiered admission. Register evidence is always used when the chip has an
    // audited scheme; without it only an explicitly requested lower tier passes.
    internal static NativeControl CreateTieredProtected(object control, bool pump, int minimum, NativeTier tier, bool pumpUnlocked)
    {
        if (minimum < (tier == NativeTier.Unrestricted ? 0 : 40) || minimum > 100) throw new InvalidOperationException("Invalid commissioning floor.");
        return CreateProtectedCore(control,pump,minimum,tier,pumpUnlocked);
    }
    private static NativeControl CreateProtectedCore(object control, bool pump, int? minimum)
    { return CreateProtectedCore(control,pump,minimum,NativeTier.Verified,false); }
    private static NativeControl CreateProtectedCore(object control, bool pump, int? minimum, NativeTier tier, bool pumpUnlocked)
    {
        BrokerService.CheckInstallation(true);
        string expected = Path.Combine(BrokerService.RuntimePath, "LibreHardwareMonitorLib.dll");
        if (control == null || !String.Equals(Path.GetFullPath(control.GetType().Assembly.Location), expected, StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Control object did not originate in the protected runtime.");
        var adapter = new NativeControl(control, pump, minimum, tier, pumpUnlocked);
        try { adapter.evidence = NativeNctEvidence.Create(control); }
        catch (NativeEvidenceUnavailableException)
        {
            if (tier == NativeTier.Verified) throw;
            adapter.evidence = null; // Behavioural safeguards only; see COOLING_BROAD_SUPPORT_PLAN.md.
            adapter.fallback = new Dictionary<string,object> {
                {"kind","lhm_fallback_v1"},{"service_identity",BrokerTransport.ServiceIdentity()},{"worker_identity",BrokerTransport.CurrentProcessIdentity()}
            };
        }
        return adapter;
    }
    // Recovery for a channel without register readback. The original firmware mode
    // is known only inside the worker that captured it, so a dead worker's channel
    // is driven to 100% and held there (until reboot) instead of being "restored"
    // from a fresh LHM context, which would replay the crashed low duty.
    internal static bool RecoverFallbackProtected(object control, Dictionary<string,object> recorded, out bool heldFull)
    {
        BrokerService.CheckInstallation(true);
        heldFull = false;
        if (recorded == null || !Object.Equals(recorded["kind"],"lhm_fallback_v1"))
            throw new InvalidOperationException("Fallback recovery requires lifetime evidence.");
        bool sameWorker = Object.Equals(recorded["worker_identity"],BrokerTransport.CurrentProcessIdentity());
        if (!sameWorker && BrokerTransport.ProcessIdentityAlive((string)recorded["worker_identity"]))
            throw new InvalidOperationException("Original native worker is still alive; recovery refused.");
        if (sameWorker) return RecoverOwned(control);
        var holder = new NativeControl(control,true,100,NativeTier.LhmSupported,false);
        holder.Acquire();   // Prepared 100%; never released in this context.
        heldFull = true;
        return true;
    }
#endif

    internal bool Owned { get { lock (reservation.Gate) return owned; } }
    internal static bool RecoverOwned(object control)
    {
        BrokerService.CheckInstallation(true);
        Reservation held;
        if (!Reservations.TryGetValue(control,out held)) return false;
        lock (held.Gate)
        {
            if (held.Owner == null) return false;
            var owner=held.Owner;
            owner.Release();
            return owner.evidence != null ? owner.FirmwareReturnVerified : owner.FirmwareReturned;
        }
    }
    // Effective tier: an unlock is reported as such; otherwise readback decides.
    internal NativeTier Tier { get { return requested == NativeTier.Unrestricted ? NativeTier.Unrestricted : evidence != null ? NativeTier.Verified : NativeTier.LhmSupported; } }
    internal bool RegisterReadback { get { return evidence != null; } }
    int INativeControlPort.Floor { get { return floor; } }
    bool INativeControlPort.Owned { get { return Owned; } }
    void INativeControlPort.Acquire() { Acquire(); }
    void INativeControlPort.SetDuty(float percent) { SetDuty(percent); }
    void INativeControlPort.Release() { Release(); }
    internal NativeControlIdentity Identity()
    {
        object sensor = control.GetType().GetProperty("Sensor").GetValue(control,null);
        Type sensorType = sensor.GetType();
        object hardware = sensorType.GetProperty("Hardware").GetValue(sensor,null);
        if (sensorType.GetProperty("SensorType").GetValue(sensor,null).ToString() != "Control" ||
            hardware.GetType().GetProperty("HardwareType").GetValue(hardware,null).ToString() != "SuperIO" ||
            !Object.ReferenceEquals(sensorType.GetProperty("Control").GetValue(sensor,null),control))
            throw new InvalidOperationException("Native journal identity is not the selected SuperIO control.");
        return new NativeControlIdentity(hardware.GetType().GetProperty("Identifier").GetValue(hardware,null).ToString(),
            sensorType.GetProperty("Identifier").GetValue(sensor,null).ToString(),pump || floor == 100,
            evidence != null ? evidence.Recovery : fallback, requested == NativeTier.Verified ? null : NativeTiers.Name(Tier));
    }
    private int Mode { get { return Convert.ToInt32(mode.GetValue(control, null)); } }
    private static Exception Cause(Exception error) { return error is TargetInvocationException && error.InnerException != null ? error.InnerException : error; }

    // The public LHM setter changes mode BEFORE updating SoftwareValue. With the
    // default cached zero this would write 0% once. Prepare while Undefined, then
    // enter Software exactly once. Re-entering also retries an identical value
    // after an event callback failed despite updating LHM's cached fields.
    private void Write(float percent)
    {
        cachedMode.SetValue(control, Enum.ToObject(mode.PropertyType, 0)); // memory only
        prime.Invoke(control, new object[] { percent });                 // memory only
        software.Invoke(control, new object[] { percent });
        if (evidence != null) evidence.VerifyDuty(percent);
        if (Mode != 1 || (float)value.GetValue(control, null) != percent)
            throw new InvalidOperationException("Native software control state did not match the request.");
    }
    internal void Acquire()
    {
        lock (reservation.Gate)
        {
            if (owned || reservation.Owner != null) throw new InvalidOperationException("Control is already reserved.");
            if (Mode != 0 && Mode != 2) throw new InvalidOperationException("Refusing an existing or unknown software-control mode.");
            reservation.Owner = this;
            returnVerified = returned = false;
            owned = true; // Track partial acquisition before the first possible hardware call.
            try { Write(100); }
            catch (Exception error) { RecoverAndThrow(error); }
        }
    }
    internal void SetDuty(float percent)
    {
        // Validate before changing any memory-backed mode or invoking callbacks.
        if (Single.IsNaN(percent) || Single.IsInfinity(percent) || percent < floor || percent > 100)
            throw new ArgumentOutOfRangeException("percent", "Duty is outside the conservative fan/pump range.");
        lock (reservation.Gate)
        {
            if (!owned || reservation.Owner != this) throw new InvalidOperationException("No owned native control session.");
            if (Mode != 1) throw new InvalidOperationException("Owned software mode was lost; explicit recovery required.");
            try { Write(percent); }
            catch (Exception error) { RecoverAndThrow(error); }
        }
    }
    private void RecoverAndThrow(Exception error)
    {
        Exception failure = Cause(error);
        try { Release(); }
        catch (Exception recovery) { throw new AggregateException("Native write and firmware restoration both failed; ownership retained for recovery.", failure, Cause(recovery)); }
        throw new InvalidOperationException("Native write failed; firmware restoration callback returned successfully.", failure);
    }
    internal void Release()
    {
        lock (reservation.Gate)
        {
            if (!owned) return;
            if (reservation.Owner != this) throw new InvalidOperationException("Control reservation was lost.");
            // SetDefault can cache Default before a throwing callback. Force a
            // memory-only transition so a second recovery really retries the I/O.
            cachedMode.SetValue(control, Enum.ToObject(mode.PropertyType, 0));
            firmware.Invoke(control, null);
            if (evidence != null) evidence.VerifyReturn();
            if (Mode != 2) throw new InvalidOperationException("Firmware restoration callback did not retain Default mode.");
            returnVerified = evidence != null;
            returned = true;
            owned = false;
            reservation.Owner = null;
        }
    }
}
