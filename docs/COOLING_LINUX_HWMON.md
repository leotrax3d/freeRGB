# Linux motherboard fan control (kernel hwmon)

On Linux, freeRGB controls the fan headers of practically every desktop
motherboard through the kernel's own sensor drivers and the
[hwmon sysfs ABI](https://docs.kernel.org/hwmon/sysfs-interface.html).
No extra driver, PawnIO or LibreHardwareMonitor is needed; freeRGB contains no
chip-specific register code. Any `pwmN` attribute with a `pwmN_enable` is a
controllable header.

## Coverage

What the kernel driver supports, freeRGB supports:

| Driver | Typical boards |
| --- | --- |
| `nct6775` (NCT6106D … NCT6799D) | Most ASUS, ASRock, MSI and many Gigabyte AM4/AM5/LGA1700 boards |
| `nct6683` | MSI boards with NCT6687D (`modprobe nct6683 force=1` on some) |
| `it87` (mainline) / out-of-tree `it87` (frankcrawford) | Gigabyte and ASRock boards with ITE IT86xx/IT87xx/IT88xx |
| `f71882fg`, `f71808e` | Fintek boards, older ASRock/MSI |
| `w83627ehf`, `w83795`, `w83793` | Winbond/older boards and servers |
| `asus-ec-sensors`, `asus_wmi_sensors` | Extra ASUS readings (mostly read-only) |
| `dell-smm-hwmon`, `thinkpad_acpi`, `gigabyte_wmi` | Dell/Lenovo/Gigabyte OEM systems where the driver exposes pwm |

CPU/GPU/NVMe temperatures (`k10temp`, `coretemp`, `amdgpu`, `nvme`, …) are
offered as curve inputs.

If no header appears, load the board's driver and make it persistent:

```bash
sudo sensors-detect            # from lm-sensors; suggests the module
sudo modprobe nct6775          # or it87, nct6683, f71882fg, …
echo nct6775 | sudo tee /etc/modules-load.d/freergb-fans.conf
```

Some boards reserve the SuperIO range through ACPI; the kernel then logs a
resource conflict. Options such as `acpi_enforce_resources=lax` or
`it87 ignore_resource_conflict=1` work around this at your own risk.

## Permissions

The daemon runs as your user. Install the udev rules, create the `freergb`
group and log in again:

```bash
sudo install -m 0644 packaging/linux/60-freergb.rules /etc/udev/rules.d/
sudo groupadd -r freergb && sudo usermod -aG freergb "$USER"
sudo udevadm control --reload-rules && sudo udevadm trigger -s hwmon -s hidraw
```

The rule grants group write access only to `pwm*` attributes of SuperIO/EC fan
drivers. The same file grants access to the motherboard RGB USB controllers.
Without it, headers are listed read-only.

## Behaviour and safeguards

* **Reading is passive.** Inventory, temperatures and RPM are read every second.
  Nothing is written until you assign a header.
* **Capture and journal.** Before the first write freeRGB records the original
  `pwmN_enable` (BIOS automatic mode) and `pwmN` value in
  `hwmon-control-journal.json`. A daemon that crashed or was killed restores
  them on its next start.
* **Return to firmware** on release, "Return all headers to BIOS", pause, normal
  daemon shutdown (SIGINT/`daemon.shutdown`) and every fault.
* **Limits.** Fans keep a 40% minimum. Headers marked as pump stay at 100%.
  0% (fan stop) and pump reduction need the global unrestricted switch
  (`cooling.support.settings`, same switch as on Windows) plus "Allow fan stop".
* **Curves** use the shared `FanCurve`/`CurveController` (hysteresis, step
  limits). A missing or stale temperature or the curve's critical temperature
  drives 100% immediately.
* **Readback and takeback.** The duty is written every second and read back.
  When the driver or BIOS switches the header out of manual mode, freeRGB
  re-asserts it once; the second time the header faults and returns to firmware.
* **Stall.** A paired tachometer at 0 RPM for 6 s while commanded ≥ 40% faults
  the header (not when fan stop is allowed).
* **No foreign takeover.** A header already in manual mode (`pwmN_enable=1`,
  e.g. fancontrol or CoolerControl) is refused.
* **Resume.** Assignments are saved in `hwmon-cooling.json`, bound to the
  board's DMI vendor/product/BIOS, and resume when the daemon starts on the same
  board and BIOS.

Fan stop, pumps and wrong label mappings remain hardware risks. Check which
header moves (watch the RPM) before you rely on a curve.

## API

| Method | Params | Result |
| --- | --- | --- |
| `cooling.hwmon.get` | – | `started`, `board`, `paused`, `unrestricted`, `floor_percent`, `chips[]` (temps, fans, pwms with `channel_id`, `writable`, `paired_fan`), `assignments[]`, `status[]` |
| `cooling.hwmon.set` | `{channel_id, mode:"manual", percent}` or `{channel_id, mode:"curve", curve, sensor_id}`, optional `pump`, `allow_stop` | `true`; needs the setup wizard's safety acknowledgement |
| `cooling.hwmon.release` | `{channel_id}` or `{}` for all | `{released}` |

hwmon providers/channels also appear in `cooling.providers.list`
(`mainboard.hwmon`), `cooling.channels.list` and `monitoring.sensors.list`;
`cooling.fans.pause` pauses them too. `--no-hwmon` disables the provider;
`--no-hardware` never starts it. `FREERGB_HWMON_ROOT` points it at a test tree.

## Validation

Unit tests run against a synthetic sysfs tree (scan, floor, pump, curve,
missing sensor, takeback, stall, journal recovery, resume, pause), plus an
end-to-end daemon run on a fake tree. Real-board acceptance (nct6775 and it87
boards at minimum) is still open; please report results with the hardware
report issue template.
