//! Coverage status per fan-controller chip, from the generated
//! `docs/fan-coverage.json` (scripts/cooling/fan-coverage.mjs). Display only:
//! admission and duty limits never depend on this table.
use serde_json::{json, Value};
use std::sync::OnceLock;

fn table() -> &'static Value {
    static TABLE: OnceLock<Value> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../../../docs/fan-coverage.json")).unwrap_or(Value::Null)
    })
}

/// Short chip name from a control id: LHM `"/lpc/<chip>/0/control/N"`,
/// hwmon `"<name>@<device path>/pwmN"`. Never returns path components.
pub fn chip_of(sensor_id: &str) -> Option<&str> {
    let chip = match sensor_id.split_once('@') {
        Some((name, _)) => name,
        None => sensor_id.split('/').nth(2)?,
    };
    (!chip.is_empty()
        && chip.len() <= 32
        && chip
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
    .then_some(chip)
}

/// `{"chip","family","tier","status"}` for a control, `None` when the chip is
/// not in the matrix. hwmon names (`nct6798`) match the LHM chip by prefix.
pub fn lookup(sensor_id: &str) -> Option<Value> {
    let chip = chip_of(sensor_id)?.to_ascii_uppercase();
    let rows = table()["chips"].as_array()?;
    let exact = rows
        .iter()
        .find(|r| r["chip"].as_str() == Some(chip.as_str()));
    let row = exact.or_else(|| {
        (chip.len() >= 6)
            .then(|| {
                rows.iter()
                    .find(|r| r["chip"].as_str().is_some_and(|c| c.starts_with(&chip)))
            })
            .flatten()
    })?;
    Some(json!({
        "chip": row["name"],
        "family": row["family"],
        "tier": row["tier"],
        "status": row["status"],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_matrix_resolves_lhm_and_hwmon_ids_without_paths() {
        let owner = lookup("/lpc/nct6701d/0/control/2").unwrap();
        assert_eq!(owner["status"], "hardware-verified");
        assert_eq!(owner["tier"], "verified");
        let ite = lookup("/lpc/it8689e/0/control/1").unwrap();
        assert_eq!(ite["status"], "simulated");
        assert_eq!(ite["family"], "ite_it87_ext");
        let hwmon = lookup("nct6798@/sys/devices/platform/nct6775.656/pwm1").unwrap();
        assert_eq!(hwmon["chip"], "Nuvoton NCT6798D");
        assert_eq!(chip_of("nct6798@/sys/devices/x/pwm1"), Some("nct6798"));
        assert_eq!(chip_of("/lpc/../etc"), None);
        assert!(lookup("/lpc/unknown9/0/control/1").is_none());
    }
}
