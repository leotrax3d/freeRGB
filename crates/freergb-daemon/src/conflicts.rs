//! Detects other RGB software that drives the same devices.
//!
//! Two programs streaming to one device at the same time locked up the
//! K70 CORE vendor interface until it was re-plugged (2026-10-18). While a
//! conflicting program runs, freergbd therefore leaves hardware alone.
//! Background services that do not stream (e.g. `SignalRgbService.exe`) are
//! not treated as conflicts: all hardware tests ran with it present.

/// Process image names (lower case) and the product they belong to.
const KNOWN: &[(&str, &str)] = &[
    ("signalrgb.exe", "SignalRGB"),
    ("signalrgblauncher.exe", "SignalRGB"),
    ("icue.exe", "Corsair iCUE"),
    ("openrgb.exe", "OpenRGB"),
    ("openrgb", "OpenRGB"),
];

/// Motherboard vendor lighting software (lower case image names). While one
/// runs freeRGB does not open motherboard RGB controllers and shows a warning;
/// other devices keep working.
const MAINBOARD_VENDORS: &[(&str, &str)] = &[
    ("armourycrate.exe", "ASUS Armoury Crate"),
    ("armourycrate.service.exe", "ASUS Armoury Crate"),
    ("armourycrateservice.exe", "ASUS Armoury Crate"),
    ("lightingservice.exe", "ASUS Aura Lighting Service"),
    ("msi center.exe", "MSI Center"),
    ("msi.centralserver.exe", "MSI Center"),
    ("mysticlight.exe", "MSI Mystic Light"),
    ("mystic_light_service.exe", "MSI Mystic Light"),
    ("rgbfusion.exe", "Gigabyte RGB Fusion"),
    ("rgbfusion2.exe", "Gigabyte RGB Fusion"),
    ("gigabytecontrolcenter.exe", "Gigabyte Control Center"),
    ("polychromergb.exe", "ASRock Polychrome RGB"),
];

/// Motherboard vendor lighting products among `processes`, sorted and de-duplicated.
pub fn mainboard_vendors_in<'a>(processes: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    lookup(MAINBOARD_VENDORS, processes)
}

/// Currently running motherboard vendor lighting software.
pub fn mainboard_vendors_running() -> Vec<&'static str> {
    process_names()
        .map(|names| mainboard_vendors_in(names.iter().map(String::as_str)))
        .unwrap_or_default()
}

/// Products of `processes` (image names) that conflict, sorted and de-duplicated.
pub fn find_in<'a>(processes: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    lookup(KNOWN, processes)
}

fn lookup<'a>(
    table: &[(&str, &'static str)],
    processes: impl IntoIterator<Item = &'a str>,
) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = processes
        .into_iter()
        .filter_map(|p| {
            let p = p.trim().to_lowercase();
            table
                .iter()
                .find(|(name, _)| p == *name)
                .map(|(_, product)| *product)
        })
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// Image names from `tasklist /FO CSV /NH` output (first CSV column).
pub fn parse_tasklist_csv(out: &str) -> Vec<String> {
    out.lines()
        .filter_map(|l| l.strip_prefix('"'))
        .filter_map(|l| l.split_once('"').map(|(name, _)| name.to_owned()))
        .collect()
}

/// Currently running conflicting products. Errors listing processes are
/// logged and treated as "no conflicts" so the daemon stays usable.
pub fn running() -> Vec<&'static str> {
    match process_names() {
        Ok(names) => find_in(names.iter().map(String::as_str)),
        Err(e) => {
            tracing::debug!(error = %e, "could not list processes");
            Vec::new()
        }
    }
}

#[cfg(windows)]
fn process_names() -> std::io::Result<Vec<String>> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    Ok(parse_tasklist_csv(&String::from_utf8_lossy(&out.stdout)))
}

#[cfg(not(windows))]
fn process_names() -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir("/proc")?.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            if let Ok(comm) = std::fs::read_to_string(entry.path().join("comm")) {
                names.push(comm.trim().to_owned());
            }
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tasklist_and_finds_conflicts() {
        let out = "\"System\",\"4\",\"Services\",\"0\",\"10 K\"\r\n\
                   \"SignalRgbService.exe\",\"5000\",\"Services\",\"0\",\"21 K\"\r\n\
                   \"SignalRgb.exe\",\"6576\",\"Console\",\"1\",\"300 K\"\r\n\
                   \"SignalRgbLauncher.exe\",\"6996\",\"Console\",\"1\",\"9 K\"\r\n\
                   \"iCUE.exe\",\"7000\",\"Console\",\"1\",\"9 K\"\r\n";
        let names = parse_tasklist_csv(out);
        assert_eq!(names.len(), 5);
        assert_eq!(names[1], "SignalRgbService.exe");
        assert_eq!(
            find_in(names.iter().map(String::as_str)),
            vec!["Corsair iCUE", "SignalRGB"]
        );
    }

    #[test]
    fn service_alone_is_not_a_conflict() {
        assert!(find_in(["SignalRgbService.exe", "explorer.exe", "freergbd.exe"]).is_empty());
        assert_eq!(find_in(["openrgb"]), vec!["OpenRGB"]);
    }

    #[test]
    fn all_header_vendors_and_services_are_matched_without_blocking_other_devices() {
        let processes = [
            "MSI.CentralServer.exe",
            "PolychromeRGB.exe",
            "RGBFusion2.exe",
            "ArmouryCrateService.exe",
            "msi.centralserver.exe",
        ];
        assert_eq!(
            mainboard_vendors_in(processes),
            vec![
                "ASRock Polychrome RGB",
                "ASUS Armoury Crate",
                "Gigabyte RGB Fusion",
                "MSI Center"
            ]
        );
        assert!(find_in(processes).is_empty());
    }

    #[test]
    fn mainboard_vendor_software_is_a_warning_not_a_global_conflict() {
        let running = [
            "ArmouryCrate.exe",
            "LightingService.exe",
            "RGBFusion.exe",
            "explorer.exe",
        ];
        assert!(find_in(running).is_empty());
        assert_eq!(
            mainboard_vendors_in(running),
            vec![
                "ASUS Armoury Crate",
                "ASUS Aura Lighting Service",
                "Gigabyte RGB Fusion"
            ]
        );
        assert!(mainboard_vendors_in(["freergbd.exe"]).is_empty());
    }
}
