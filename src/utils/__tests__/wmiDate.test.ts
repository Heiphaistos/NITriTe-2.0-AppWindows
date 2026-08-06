import { describe, it, expect } from "vitest";
import { formatWmiDateTime } from "@/utils/wmiDate";

describe("formatWmiDateTime", () => {
  it("parses a real DMTF timestamp captured from Win32_ShadowCopy.InstallDate", () => {
    // Capturé en direct : Get-WmiObject Win32_ShadowCopy | Select InstallDate
    expect(formatWmiDateTime("20260803162919.545193+120")).toBe("03/08/2026 16:29");
  });

  it("parses a real DMTF timestamp captured from Get-ComputerRestorePoint.CreationTime", () => {
    expect(formatWmiDateTime("20260803142911.024981-000")).toBe("03/08/2026 14:29");
  });

  it("does not silently produce 'Invalid Date' the way new Date() alone does on DMTF input", () => {
    const raw = "20260803162919.545193+120";
    // Preuve du bug corrigé : new Date() seul echoue silencieusement sur ce format.
    expect(isNaN(new Date(raw).getTime())).toBe(true);
    expect(formatWmiDateTime(raw)).not.toContain("Invalid Date");
  });

  it("returns an em dash for empty input", () => {
    expect(formatWmiDateTime("")).toBe("—");
  });

  it("falls back to standard Date parsing for a non-DMTF (e.g. ISO) string", () => {
    const out = formatWmiDateTime("2026-08-03T16:29:19Z");
    expect(out).not.toBe("—");
    expect(out.toLowerCase()).not.toContain("invalid");
  });

  it("returns the raw string unchanged if it cannot be parsed at all", () => {
    expect(formatWmiDateTime("not a date")).toBe("not a date");
  });
});
