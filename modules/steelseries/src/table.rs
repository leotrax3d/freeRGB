// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
}
/// Sub-controller protocols ported so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proto {
    /// upstream: SteelSeriesRivalController.h steelseries_type RIVAL_100
    Rival100,
    /// upstream: SteelSeriesRivalController.h steelseries_type RIVAL_300
    Rival300,
    /// upstream: SteelSeriesRivalController.h steelseries_type RIVAL_600
    Rival600,
    /// upstream: SteelSeriesRivalController.h steelseries_type RIVAL_650
    Rival650,
    /// upstream: SteelSeriesRivalController.h steelseries_type RIVAL_700
    Rival700,
    /// upstream: SteelSeriesRivalController.h steelseries_type RIVAL_3
    Rival3,
    /// upstream: SteelSeriesRivalController.h steelseries_type RIVAL_5
    Rival5,
    /// upstream: SteelSeriesRivalController.h steelseries_type SENSEI
    Sensei,
    /// upstream: SteelSeriesRivalController.h steelseries_type SIBERIA_350
    Siberia350,
    /// Arctis 5 driver (own controller class, not in the old steelseries_type
    /// enum): upstream SteelSeriesArctis5Controller
    Arctis5,
    /// Arctis Nova 3 driver: upstream SteelSeriesArctisNova3Controller
    ArctisNova3,
    /// upstream: SteelSeriesRivalController.h steelseries_type AEROX_3
    Aerox3,
    /// upstream: SteelSeriesRivalController.h steelseries_type AEROX_5
    Aerox5,
    /// upstream: SteelSeriesAeroxWirelessController - one controller class
    /// for the Aerox 3/5/9 wireless+wired rows and Rival 3 Wireless
    AeroxWireless {
        /// upstream: IsWireless() - flag byte addition and readback
        wireless: bool,
        /// upstream: GetMouse() aerox_9 table (4th "Reactive" LED)
        aerox9: bool,
        /// upstream: GetMouse() rival_3_wireless table (single LED, own cmd)
        rival3: bool,
    },
}
/// Maps an upstream detector callback (REGISTER_HID_DETECTOR_* entry in
/// SteelSeriesControllerDetect.cpp) to its ported protocol.
/// upstream: SteelSeriesControllerDetect.cpp lines 489-509 (Rival handlers)
pub fn proto_of_handler(handler: &str) -> Option<Proto> {
    match handler {
        "DetectSteelSeriesRival100" => Some(Proto::Rival100),
        "DetectSteelSeriesRival300" => Some(Proto::Rival300),
        "DetectSteelSeriesRival600" => Some(Proto::Rival600),
        "DetectSteelSeriesRival650" => Some(Proto::Rival650),
        "DetectSteelSeriesRival700" => Some(Proto::Rival700),
        // upstream: SteelSeriesControllerDetect.cpp DetectSteelSeriesRival3
        // (interface 3 rows, old-firmware 0x1824 and 0x184C)
        "DetectSteelSeriesRival3" => Some(Proto::Rival3),
        // upstream: SteelSeriesControllerDetect.cpp DetectSteelSeriesRival5
        // (interface 0, usage page 0xFFC0, usage 1)
        "DetectSteelSeriesRival5" => Some(Proto::Rival5),
        // upstream: SteelSeriesControllerDetect.cpp DetectSteelSeriesSensei
        // (Sensei TEN, Sensei 310, Rival 310 rows, interface 0)
        "DetectSteelSeriesSensei" => Some(Proto::Sensei),
        // upstream: SteelSeriesControllerDetect.cpp DetectSteelSeriesHeadset
        // (Siberia 350, interface 3)
        "DetectSteelSeriesHeadset" => Some(Proto::Siberia350),
        // upstream: SteelSeriesControllerDetect.cpp DetectSteelSeriesArctis5
        // (interface 5, PIDs 0x1250/0x12AA)
        "DetectSteelSeriesArctis5" => Some(Proto::Arctis5),
        // upstream: SteelSeriesControllerDetect.cpp DetectSteelSeriesArctisNova3
        // (interface 4, usage page 0xFFC0, usage 1)
        "DetectSteelSeriesArctisNova3" => Some(Proto::ArctisNova3),
        // upstream: SteelSeriesControllerDetect.cpp DetectSteelSeriesAerox3 /
        // DetectSteelSeriesAerox5 (wired, interface 3, usage page 0xFFC0, usage 1)
        "DetectSteelSeriesAerox3" => Some(Proto::Aerox3),
        "DetectSteelSeriesAerox5" => Some(Proto::Aerox5),
        // upstream: DetectSteelSeriesAeroxWireless family, all interface 3,
        // usage page 0xFFC0, usage 1; wireless rows set the 0x40 flag and do
        // the hid_read_timeout readback after each write
        "DetectSteelSeriesAerox3Wireless" => Some(Proto::AeroxWireless {
            wireless: true,
            aerox9: false,
            rival3: false,
        }),
        "DetectSteelSeriesAerox3WirelessWired" => Some(Proto::AeroxWireless {
            wireless: false,
            aerox9: false,
            rival3: false,
        }),
        "DetectSteelSeriesAerox3Gen2Wireless" => Some(Proto::AeroxWireless {
            wireless: true,
            aerox9: false,
            rival3: false,
        }),
        "DetectSteelSeriesAerox5Wireless"
        | "DetectSteelSeriesAerox5WirelessWired"
        | "DetectSteelSeriesAerox5DestinyWireless"
        | "DetectSteelSeriesAerox5DestinyWirelessWired"
        | "DetectSteelSeriesAerox5DiabloWireless"
        | "DetectSteelSeriesAerox5DiabloWirelessWired" => Some(Proto::AeroxWireless {
            wireless: !handler.ends_with("Wired"),
            aerox9: false,
            rival3: false,
        }),
        "DetectSteelSeriesAerox9Wireless" => Some(Proto::AeroxWireless {
            wireless: true,
            aerox9: true,
            rival3: false,
        }),
        "DetectSteelSeriesAerox9WirelessWired" => Some(Proto::AeroxWireless {
            wireless: false,
            aerox9: true,
            rival3: false,
        }),
        // upstream: IsWireless() returns false for RIVAL_3_WIRELESS; it uses
        // its own 0x13 command instead of the 0x40-flagged 0x21 packets
        "DetectSteelSeriesRival3Wireless" => Some(Proto::AeroxWireless {
            wireless: false,
            aerox9: false,
            rival3: true,
        }),
        _ => None,
    }
}
pub fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = detector(i);
    let ported = d.and_then(|d| proto_of_handler(d.handler));
    if let (Some(d), Some(_)) = (d, ported) {
        return Identification {
            supported: true,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Mouse),
            reason: None,
        };
    }
    Identification {
        // Detector inventory is not a protocol implementation: OpenRGB
        // dispatches these entries to 17 different controllers.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "SteelSeries detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no SteelSeries HID detector for this VID/PID".into()
        }),
    }
}
