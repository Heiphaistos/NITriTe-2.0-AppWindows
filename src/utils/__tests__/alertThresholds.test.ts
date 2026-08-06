import { describe, it, expect } from "vitest";
import { normalizeThresholds } from "@/utils/alertThresholds";
import type { AlertThresholds } from "@/components/ui/AlertThresholdsModal.vue";

const DEFAULTS: AlertThresholds = {
  cpu_warn: 75, cpu_crit: 90,
  ram_warn: 75, ram_crit: 90,
  disk_warn: 85, disk_crit: 95,
};

describe("normalizeThresholds", () => {
  it("leaves already-valid thresholds untouched", () => {
    const out = normalizeThresholds(DEFAULTS, DEFAULTS);
    expect(out).toEqual(DEFAULTS);
  });

  it("swaps a pair when critical was entered lower than warning", () => {
    // Reproduit le bug reel : DashboardPage.vue::computeHealthScore()/checkThresholdAlerts()
    // testent toujours "if (x >= crit) ... else if (x >= warn) ..." — si crit < warn,
    // le palier "avertissement" est mathematiquement inatteignable (tout depassement du
    // seuil crit, plus bas, est immediatement classe critique).
    const bad: AlertThresholds = { ...DEFAULTS, cpu_warn: 90, cpu_crit: 75 };
    const out = normalizeThresholds(bad, DEFAULTS);
    expect(out.cpu_warn).toBeLessThanOrEqual(out.cpu_crit);
    expect(out.cpu_warn).toBe(75);
    expect(out.cpu_crit).toBe(90);
  });

  it("swaps independently for each of the 3 metric pairs", () => {
    const bad: AlertThresholds = {
      cpu_warn: 95, cpu_crit: 60,
      ram_warn: 88, ram_crit: 70,
      disk_warn: 99, disk_crit: 51,
    };
    const out = normalizeThresholds(bad, DEFAULTS);
    expect(out.cpu_warn).toBeLessThanOrEqual(out.cpu_crit);
    expect(out.ram_warn).toBeLessThanOrEqual(out.ram_crit);
    expect(out.disk_warn).toBeLessThanOrEqual(out.disk_crit);
  });

  it("clamps values outside the [50,100] range", () => {
    const bad: AlertThresholds = { ...DEFAULTS, cpu_warn: 5, cpu_crit: 500 };
    const out = normalizeThresholds(bad, DEFAULTS);
    expect(out.cpu_warn).toBe(50);
    expect(out.cpu_crit).toBe(100);
  });

  it("falls back to the previous value on NaN (empty input field)", () => {
    const bad: AlertThresholds = { ...DEFAULTS, ram_warn: NaN };
    const out = normalizeThresholds(bad, DEFAULTS);
    expect(out.ram_warn).toBe(DEFAULTS.ram_warn);
  });

  it("rounds fractional input", () => {
    const bad: AlertThresholds = { ...DEFAULTS, disk_warn: 77.6 };
    const out = normalizeThresholds(bad, DEFAULTS);
    expect(out.disk_warn).toBe(78);
  });
});
