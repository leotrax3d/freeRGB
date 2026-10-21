// Version-pinned native adapter. Only the protected fixed100 diagnostic invokes it.
// All calls must stay in a protected isolated worker, never in the GUI/RPC loop.
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
    private bool owned;
    private INativeControlEvidence evidence;
    private bool returnVerified;
    public bool FirmwareReturnVerified { get { lock (reservation.Gate) return returnVerified && !owned; } }
    public bool JournalCleared { get { return false; } } // Only the journal wrapper can assert this.

    private NativeControl(object control, bool pump)
    {
        if (control == null) throw new ArgumentNullException("control");
        Type type = control.GetType();
        AssemblyName assembly = type.Assembly.GetName();
        if (type.FullName != "LibreHardwareMonitor.Hardware.Control" || assembly.Name != "LibreHardwareMonitorLib" || assembly.Version != new Version(0, 9, 6, 0))
            throw new InvalidOperationException("Native control requires the pinned LHM 0.9.6 control implementation.");
        this.control = control;
        floor = pump ? 100 : 40;
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
#else
    internal static NativeControl CreateProtected(object control, bool pump)
    {
        BrokerService.CheckInstallation(true);
        string expected = Path.Combine(BrokerService.RuntimePath, "LibreHardwareMonitorLib.dll");
        if (control == null || !String.Equals(Path.GetFullPath(control.GetType().Assembly.Location), expected, StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Control object did not originate in the protected runtime.");
        var adapter = new NativeControl(control, pump);
        adapter.evidence = NativeNctEvidence.Create(control);
        return adapter;
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
            return owner.FirmwareReturnVerified;
        }
    }
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
            sensorType.GetProperty("Identifier").GetValue(sensor,null).ToString(),floor == 100,
            evidence == null ? null : evidence.Recovery);
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
            returnVerified = false;
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
            owned = false;
            reservation.Owner = null;
        }
    }
}
