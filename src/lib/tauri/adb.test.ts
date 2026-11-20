import { describe, expect, it } from "vitest";
import { parseAdtError } from "./adb";

describe("parseAdtError", () => {
  it("extracts structured AdtError payloads from Tauri IPC rejections", () => {
    const parsed = parseAdtError({
      code: "device_unauthorized",
      message: "Device is unauthorized.",
      guidance: "Unlock your phone and accept the RSA USB debugging prompt.",
    });

    expect(parsed).toEqual({
      code: "device_unauthorized",
      message: "Device is unauthorized.",
      guidance: "Unlock your phone and accept the RSA USB debugging prompt.",
    });
  });

  it("normalizes non-string guidance fields to null", () => {
    const parsed = parseAdtError({
      code: "timeout",
      message: "ADB command timed out.",
      guidance: 42,
    });

    expect(parsed).toEqual({
      code: "timeout",
      message: "ADB command timed out.",
      guidance: null,
    });
  });

  it("wraps standard Error instances and primitive rejections safely", () => {
    expect(parseAdtError(new Error("IPC disconnected"))).toEqual({
      code: "unknown_error",
      message: "IPC disconnected",
      guidance: null,
    });

    expect(parseAdtError("raw string failure")).toEqual({
      code: "unknown_error",
      message: "raw string failure",
      guidance: null,
    });
  });
});
