import { describe, expect, it } from "vitest";
import { DEFAULT_CURVE } from "@/lib/api/control";
import { hwmonRequest, hwmonTemperatures, type HwmonView } from "./hwmonModel";

const view = { chips: [{ id: "nct6798@pci", name: "nct6798", read_only_hint: false, temps: [{ index: 1, label: "CPUTIN", celsius: 41.4 }], fans: [], pwms: [] }] } as unknown as HwmonView;

describe("hwmon model", () => {
  it("lists temperatures as curve inputs", () => {
    expect(hwmonTemperatures(view)).toEqual([{ value: "nct6798@pci/temp1", label: "nct6798: CPUTIN (41 °C)" }]);
  });
  it("maps modes to RPC calls", () => {
    expect(hwmonRequest("c/pwm1", "firmware", 0, "", undefined, false, false)?.method).toBe("cooling.hwmon.release");
    expect(hwmonRequest("c/pwm1", "manual", 140, "", undefined, true, false)?.params.percent).toBe(100);
    expect(hwmonRequest("c/pwm1", "curve", 0, "", DEFAULT_CURVE, false, false)).toBeNull();
    expect(hwmonRequest("c/pwm1", "curve", 0, "c/temp1", DEFAULT_CURVE, false, false)?.params.sensor_id).toBe("c/temp1");
  });
});
