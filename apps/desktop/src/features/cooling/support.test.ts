import { describe, expect, it } from "vitest";
import type { CoolingChannel } from "@/lib/api/types";
import { channelTier, controlRefusal, coverageLabel, tierLabel, tierTone } from "./support";

const control = (support: CoolingChannel["support"], available = true): CoolingChannel => ({
  provider: "mainboard.lhm", id: "x", device_id: "d", name: "Fan Control #1", kind: "control",
  reading: { id: "/lpc/it8689e/0/control/0", name: "Fan Control #1", unit: "%", value: 50, age_ms: 0 },
  control_available: available, support,
});

describe("support tiers", () => {
  it("labels every tier and never colours an unknown tier as safe", () => {
    expect(tierLabel("verified")).toBe("Verified");
    expect(tierLabel("lhm_supported")).toBe("LHM-supported");
    expect(tierLabel("unrestricted")).toBe("Unrestricted");
    expect(tierTone("verified")).toBe("ok");
    expect(tierTone("unrestricted")).toBe("warn");
    expect(tierTone(undefined)).toBe("idle");
  });

  it("derives the channel capability from the protected inventory", () => {
    expect(channelTier(control({ tier: "lhm_supported", register_readback: false, software_mode: false, held_full: false }))).toBe("lhm_supported");
    expect(channelTier(control(null, false))).toBe("read_only");
    expect(channelTier(control(undefined))).toBeNull();
  });

  it("explains refusals for foreign software mode and crash fallback", () => {
    expect(controlRefusal({ tier: "verified", register_readback: true, software_mode: true, held_full: false })).toMatch(/will not take it over/);
    expect(controlRefusal({ tier: "lhm_supported", register_readback: false, software_mode: false, held_full: true })).toMatch(/Restart Windows/);
    expect(controlRefusal(null)).toBeNull();
  });
});

describe("coverage label", () => {
  it("names the chip and never presents simulation as hardware evidence", () => {
    expect(coverageLabel(null)).toBeNull();
    expect(coverageLabel({ chip: "Nuvoton NCT6701D", family: "nct6701d", tier: "verified", status: "hardware-verified" })).toBe("Nuvoton NCT6701D · hardware-verified");
    expect(coverageLabel({ chip: "ITE IT8689E", family: "ite_it87_ext", tier: "lhm-supported", status: "simulated" })).toBe("ITE IT8689E · simulated only");
  });
});
