// Native boundary only: checked register reads for the pinned LHM Nct677X path.
// No arbitrary register addresses/values are accepted from clients.
using System;
using System.Collections.Generic;
using System.Reflection;

internal sealed class NativeNctEvidence : INativeControlEvidence
{
    private readonly object chip;
    private readonly MethodInfo read, wait, release;
    private readonly ushort modeRegister, commandRegister;
    private readonly byte originalMode, originalCommand;
    private readonly Dictionary<string,object> recovery;
    private NativeNctEvidence(object control, Dictionary<string,object> recorded)
    {
        object sensor = control.GetType().GetProperty("Sensor").GetValue(control,null);
        object hardware = sensor.GetType().GetProperty("Hardware").GetValue(sensor,null);
        Type hardwareType = hardware.GetType();
        if (hardwareType.FullName != "LibreHardwareMonitor.Hardware.Motherboard.SuperIOHardware")
            throw new InvalidOperationException("No audited native register evidence for this hardware backend.");
        FieldInfo source = hardwareType.GetField("_superIO",BindingFlags.NonPublic|BindingFlags.Instance);
        chip = source == null ? null : source.GetValue(hardware);
        if (chip == null || chip.GetType().FullName != "LibreHardwareMonitor.Hardware.Motherboard.Lpc.Nct677X")
            throw new InvalidOperationException("Native register verification currently requires the audited Nct677X backend.");
        Type type = chip.GetType();
        string name = type.GetProperty("Chip").GetValue(chip,null).ToString();
        // These use separate per-channel mode/command registers in pinned 0.9.6.
        // Shared-bit/EC/curve schemes (NCT668x) are deliberately excluded.
        if (Array.IndexOf(new[] { "NCT6771F","NCT6776F","NCT6779D","NCT6791D","NCT6792D","NCT6792DA","NCT6793D","NCT6795D","NCT6796D","NCT6796DR","NCT6796DS","NCT6797D","NCT6798D","NCT6799D","NCT6701D","NCT5585D","NCT610XD" },name) < 0)
            throw new InvalidOperationException("Native register scheme is not audited for this chip.");
        if (!(bool)Field(type,"_isNuvotonVendor").GetValue(chip)) throw new InvalidOperationException("Native vendor check failed.");
        int index = Convert.ToInt32(sensor.GetType().GetProperty("Index").GetValue(sensor,null));
        var modes = (ushort[])Field(type,"FAN_CONTROL_MODE_REG").GetValue(chip);
        var commands = (ushort[])Field(type,"FAN_PWM_COMMAND_REG").GetValue(chip);
        if (index < 0 || index >= modes.Length || index >= commands.Length || index >= 7)
            throw new InvalidOperationException("Control index is outside the audited native register arrays.");
        modeRegister = modes[index]; commandRegister = commands[index];
        read = type.GetMethod("ReadByte",BindingFlags.NonPublic|BindingFlags.Instance,null,new[] { typeof(ushort) },null);
        Type mutexes = type.Assembly.GetType("LibreHardwareMonitor.Hardware.Mutexes",true);
        var bus = mutexes.GetField("_isaBusMutex",BindingFlags.NonPublic|BindingFlags.Static);
        if (bus == null || bus.GetValue(null) == null)
            throw new InvalidOperationException("Native ISA mutex was not initialized; no verified I/O is possible.");
        wait = mutexes.GetMethod("WaitIsaBus",BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Static);
        release = mutexes.GetMethod("ReleaseIsaBus",BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Static);
        if (read == null || read.ReturnType != typeof(byte) || wait == null || release == null)
            throw new InvalidOperationException("Pinned native readback metadata changed.");
        byte[] baseline = ReadPair();
        originalMode = recorded == null ? baseline[0] : checked((byte)(int)recorded["mode"]);
        originalCommand = recorded == null ? baseline[1] : checked((byte)(int)recorded["command"]);
        if (originalMode == 0) throw new InvalidOperationException("Channel is already in native manual mode; ownership cannot be established.");
        recovery = new Dictionary<string,object> {
            {"kind","lhm_nct677x_registers_v1"},{"chip",name},{"index",index},
            {"port",Convert.ToInt32(Field(type,"_port").GetValue(chip))},
            {"revision",Convert.ToInt32(Field(type,"_revision").GetValue(chip))},
            {"mode",(int)originalMode},{"command",(int)originalCommand}
        };
        if (recorded != null)
            foreach (string key in new[] { "kind","chip","index","port","revision" })
                if (!recorded.ContainsKey(key) || !Object.Equals(recorded[key],recovery[key])) throw new InvalidOperationException("Native recovery hardware identity changed.");
        recovery.Add("service_identity",BrokerTransport.ServiceIdentity());
        recovery.Add("worker_identity",BrokerTransport.CurrentProcessIdentity());
    }
    private static FieldInfo Field(Type type, string name)
    {
        var field = type.GetField(name,BindingFlags.NonPublic|BindingFlags.Instance);
        if (field == null) throw new InvalidOperationException("Pinned native field changed: " + name);
        return field;
    }
    internal static NativeNctEvidence Create(object control)
    {
        BrokerService.CheckInstallation(true);
        return new NativeNctEvidence(control,null);
    }
    internal static bool RecoverProtected(object control, Dictionary<string,object> recorded)
    {
        BrokerService.CheckInstallation(true);
        if (recorded == null || !recorded.ContainsKey("worker_identity") || !recorded.ContainsKey("service_identity"))
            throw new InvalidOperationException("Legacy intent has no worker lifetime evidence; automatic replay refused.");
        bool sameWorker=Object.Equals(recorded["worker_identity"],BrokerTransport.CurrentProcessIdentity());
        if (!sameWorker && BrokerTransport.ProcessIdentityAlive((string)recorded["worker_identity"]))
            throw new InvalidOperationException("Original native worker is still alive; recovery refused.");
        var evidence=new NativeNctEvidence(control,recorded);
        if (sameWorker)
        {
            if (NativeControl.RecoverOwned(control)) { evidence.VerifyReturn(); return true; }
            if (evidence.ReadPair()[0] == evidence.originalMode) return true;
            throw new InvalidOperationException("Owned native context could not be verified for recovery.");
        }
        if (evidence.ReadPair()[0] == evidence.originalMode) return true; // Already returned; no register replay.
        if (!Object.Equals(recorded["service_identity"],BrokerTransport.ServiceIdentity()))
            throw new InvalidOperationException("Service/boot lifetime changed while native mode differs; automatic replay refused.");
        Type type=evidence.chip.GetType();
        int index=(int)recorded["index"];
        if (!(bool)evidence.wait.Invoke(null,new object[] { 100 })) throw new InvalidOperationException("Recovery could not acquire ISA bus.");
        try
        {
            // Only the selected original restore cache, never a fresh context's
            // already-overridden baseline. No client-supplied register addresses.
            ((byte[])Field(type,"_initialFanControlMode").GetValue(evidence.chip))[index]=evidence.originalMode;
            ((byte[])Field(type,"_initialFanPwmCommand").GetValue(evidence.chip))[index]=evidence.originalCommand;
            ((bool[])Field(type,"_restoreDefaultFanControlRequired").GetValue(evidence.chip))[index]=true;
            type.GetMethod("SetControl").Invoke(evidence.chip,new object[] { index,null });
            evidence.VerifyReturn();
            return true;
        }
        finally { evidence.release.Invoke(null,null); }
    }
    private byte[] ReadPair()
    {
        // LHM setters may silently return on ISA-bus contention. A callback or
        // cached SoftwareValue is therefore insufficient evidence of a write.
        if (!(bool)wait.Invoke(null,new object[] { 100 })) throw new InvalidOperationException("Native register readback could not acquire the ISA bus.");
        try { return new[] { (byte)read.Invoke(chip,new object[] { modeRegister }), (byte)read.Invoke(chip,new object[] { commandRegister }) }; }
        finally { release.Invoke(null,null); }
    }
    public Dictionary<string,object> Recovery { get { return new Dictionary<string,object>(recovery); } }
    public void VerifyDuty(float percent)
    {
        byte[] actual = ReadPair();
        if (actual[0] != 0 || actual[1] != (byte)(percent * 2.55f))
            throw new InvalidOperationException("Native mode/command register readback does not match the requested duty.");
    }
    public void VerifyReturn()
    {
        byte[] actual = ReadPair();
        // Automatic firmware may already have updated its command byte; do not
        // confuse that legitimate change with failed restoration of its mode.
        if (actual[0] != originalMode) throw new InvalidOperationException("Original native firmware mode was not restored.");
    }
}
