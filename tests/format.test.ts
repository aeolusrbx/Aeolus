import { describe, expect, it } from "vitest";
import { displayValue, flagTypeOf, formatBytes, formatDuration, isHexColor, lighten, parseFlagInput, relativeTime } from "../src/utils/format";

describe("format utils", () => {
  it("formats bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(412_000_000)).toBe("393 MB");
    expect(formatBytes(null)).toBe("—");
  });

  it("formats durations", () => {
    expect(formatDuration(42)).toBe("42s");
    expect(formatDuration(125)).toBe("2m 5s");
    expect(formatDuration(3720)).toBe("1h 2m");
    expect(formatDuration(-1)).toBe("—");
  });

  it("formats relative time", () => {
    const now = new Date("2026-11-11T12:00:00Z");
    expect(relativeTime("2026-11-11T11:59:50Z", now)).toBe("just now");
    expect(relativeTime("2026-11-11T11:30:00Z", now)).toBe("30 min ago");
    expect(relativeTime(null, now)).toBe("never");
  });

  it("validates and lightens colors", () => {
    expect(isHexColor("#FF861F")).toBe(true);
    expect(isHexColor("orange")).toBe(false);
    expect(lighten("#000000", 0.5)).toBe("#808080");
  });

  it("detects flag types and parses input", () => {
    expect(flagTypeOf("DFIntTextureQualityOverride")).toBe("integer");
    expect(flagTypeOf("FFlagDebugSkyGray")).toBe("boolean");
    expect(flagTypeOf("Nope")).toBe("unknown");
    expect(parseFlagInput("True", "boolean")).toBe(true);
    expect(parseFlagInput("42", "integer")).toBe(42);
    expect(parseFlagInput("4.2", "integer")).toBe("4.2");
    expect(parseFlagInput(" x ", "string")).toBe(" x ");
    expect(displayValue(true)).toBe("true");
    expect(displayValue("a")).toBe("a");
  });
});
