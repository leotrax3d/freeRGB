//! Support tiers for motherboard fan channels (DECISIONS.md #14) and the duty
//! limits each tier permits. Pure policy: no hardware access, no persistence.
//!
//! * `Verified`: audited register readback (today: Nuvoton separate-mode chips).
//! * `LhmSupported`: any LHM SuperIO control with a software mode; reduced duty
//!   only after a passed guided channel test, guarded by RPM plausibility.
//! * `Unrestricted`: explicit per-channel unlock on untested hardware; fan stop
//!   allowed, plausibility deviations are warnings only.
//! * `ReadOnly`: sensors without a usable control.
use crate::channels::ChannelRole;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTier {
    Verified,
    LhmSupported,
    Unrestricted,
    ReadOnly,
}

impl SupportTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::LhmSupported => "lhm_supported",
            Self::Unrestricted => "unrestricted",
            Self::ReadOnly => "read_only",
        }
    }
    /// Tier name passed to the protected service; `None` keeps the legacy,
    /// verified-only authorization record.
    pub fn protected_name(self) -> Option<&'static str> {
        match self {
            Self::LhmSupported | Self::Unrestricted => Some(self.as_str()),
            Self::Verified | Self::ReadOnly => None,
        }
    }
}

/// Lowest duty a fan may receive without an unrestricted unlock.
pub const SAFE_FAN_FLOOR: u8 = 40;

/// What the selected channel may do, given its capability and user evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ChannelLimits {
    pub tier: SupportTier,
    pub floor_percent: u8,
    /// Fan stop (0 RPM) is expected rather than a fault.
    pub allow_stop: bool,
    /// A pump that left 100% after its separate confirmation.
    pub pump_reduction: bool,
    /// RPM plausibility deviations return the channel to firmware (otherwise warn).
    pub plausibility_fatal: bool,
}

/// Evidence the daemon holds for one channel identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelEvidence {
    /// The user assigned a role (fan/pump) to this channel.
    pub role: Option<ChannelRole>,
    pub test_passed: bool,
    /// Unrestricted unlock is valid for the current board/chip/channel identity.
    pub unlocked: bool,
    /// Second, separate confirmation for an unlocked pump.
    pub pump_confirmed: bool,
}

/// Duty limits for normal curve operation. `capability` is what the protected
/// inventory reports for the chip (verified or LHM-supported).
pub fn limits(capability: SupportTier, evidence: ChannelEvidence) -> Result<ChannelLimits, String> {
    if !matches!(
        capability,
        SupportTier::Verified | SupportTier::LhmSupported
    ) {
        return Err("Channel has no controllable LHM software mode".into());
    }
    if evidence.unlocked {
        let pump = evidence.role == Some(ChannelRole::Pump);
        let reduce = !pump || evidence.pump_confirmed;
        return Ok(ChannelLimits {
            tier: SupportTier::Unrestricted,
            floor_percent: if reduce { 0 } else { 100 },
            allow_stop: reduce,
            pump_reduction: pump && reduce,
            plausibility_fatal: false,
        });
    }
    let fan = evidence.role == Some(ChannelRole::Fan);
    // Pumps and channels without an assigned role are never reduced.
    let reduce = fan && (capability == SupportTier::Verified || evidence.test_passed);
    Ok(ChannelLimits {
        tier: capability,
        floor_percent: if reduce { SAFE_FAN_FLOOR } else { 100 },
        allow_stop: false,
        pump_reduction: false,
        plausibility_fatal: capability == SupportTier::LhmSupported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(
        role: Option<ChannelRole>,
        test: bool,
        unlocked: bool,
        pump: bool,
    ) -> ChannelEvidence {
        ChannelEvidence {
            role,
            test_passed: test,
            unlocked,
            pump_confirmed: pump,
        }
    }

    #[test]
    fn untested_unassigned_and_pump_channels_are_never_reduced() {
        for capability in [SupportTier::Verified, SupportTier::LhmSupported] {
            for role in [None, Some(ChannelRole::Pump)] {
                let l = limits(capability, evidence(role, true, false, true)).unwrap();
                assert_eq!(l.floor_percent, 100, "{capability:?} {role:?}");
                assert!(!l.allow_stop && !l.pump_reduction);
            }
        }
        let untested = limits(
            SupportTier::LhmSupported,
            evidence(Some(ChannelRole::Fan), false, false, false),
        )
        .unwrap();
        assert_eq!(untested.floor_percent, 100);
        assert!(untested.plausibility_fatal);
    }

    #[test]
    fn verified_or_tested_fans_keep_the_forty_percent_floor() {
        let verified = limits(
            SupportTier::Verified,
            evidence(Some(ChannelRole::Fan), false, false, false),
        )
        .unwrap();
        assert_eq!(
            (
                verified.tier,
                verified.floor_percent,
                verified.plausibility_fatal
            ),
            (SupportTier::Verified, 40, false)
        );
        let tested = limits(
            SupportTier::LhmSupported,
            evidence(Some(ChannelRole::Fan), true, false, false),
        )
        .unwrap();
        assert_eq!(
            (tested.tier, tested.floor_percent, tested.allow_stop),
            (SupportTier::LhmSupported, 40, false)
        );
    }

    #[test]
    fn unlock_allows_stop_and_needs_a_second_pump_confirmation() {
        for capability in [SupportTier::Verified, SupportTier::LhmSupported] {
            for role in [None, Some(ChannelRole::Fan)] {
                let l = limits(capability, evidence(role, false, true, false)).unwrap();
                assert_eq!((l.tier, l.floor_percent), (SupportTier::Unrestricted, 0));
                assert!(l.allow_stop && !l.plausibility_fatal);
            }
            let pump = limits(
                capability,
                evidence(Some(ChannelRole::Pump), false, true, false),
            )
            .unwrap();
            assert_eq!((pump.floor_percent, pump.pump_reduction), (100, false));
            let confirmed = limits(
                capability,
                evidence(Some(ChannelRole::Pump), false, true, true),
            )
            .unwrap();
            assert_eq!(
                (confirmed.floor_percent, confirmed.pump_reduction),
                (0, true)
            );
        }
        assert!(limits(SupportTier::ReadOnly, evidence(None, true, true, true)).is_err());
        assert!(limits(SupportTier::Unrestricted, evidence(None, true, true, true)).is_err());
    }

    #[test]
    fn protected_names_keep_the_legacy_record_for_verified() {
        assert_eq!(SupportTier::Verified.protected_name(), None);
        assert_eq!(
            SupportTier::LhmSupported.protected_name(),
            Some("lhm_supported")
        );
        assert_eq!(
            SupportTier::Unrestricted.protected_name(),
            Some("unrestricted")
        );
    }
}
