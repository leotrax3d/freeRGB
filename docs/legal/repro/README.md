# Nicht ausgeführte Fehlerregressionen

`gpu_recovery.rs` verwendet ausschließlich einen eigenen in-memory-Backend und eigens angelegte temporäre Dateien. Kein `platform_backends()`, NVML-, Treiber- oder Hardwarezugriff. Die beiden Tests prüfen offene Fehler FR-001/002 und sollen am geprüften Stand **fehlschlagen**. Sie wurden hier mangels Rust-Toolchain nicht kompiliert oder ausgeführt; erwartetes Ergebnis ist ausdrücklich kein Laufzeitnachweis.

In einem separaten Checkout mit Rust und den Projektabhängigkeiten:

```bash
cp docs/legal/repro/gpu_recovery.rs crates/freergb-daemon/tests/legal_audit_gpu_recovery.rs
cargo test -p freergb-daemon --test legal_audit_gpu_recovery --locked
```

Erst nach überprüften Produktkorrekturen sollen beide Tests bestehen. Der Test liegt absichtlich außerhalb der normalen Testsuite, damit ein reiner Audit-Dokumentationscommit nicht absichtlich bestehende CI rot schaltet. Bei Übernahme in die reguläre Suite den Dateiinhalt und die API-Kompatibilität am tatsächlichen Fixstand kontrollieren. Kein Test behauptet eine reale Temperatur-/Firmwarewirkung.

Weitere nötige Fault-Injection-Fälle: nicht erreichbares GPU-Gerät bei Recovery, Fehlermeldung beim Wiederholungsversuch, hwmon-Fehler beim `pwmN_enable`-Restore, ENOSPC vor Übernahme, API3-Fail-safe-Transfer mit `Ok(0)`/`Ok(len-1)`, Worker-/Treiberblockade und Widerruf bei laufendem Kanal. Nur Simulationen/Mocks ohne zusätzliche Hardwarefreigabe.
