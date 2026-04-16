import { describe, expect, it } from "bun:test";
import {
  parseBackendValidationMode,
  resolveBackendValidationSettings,
} from "./backendValidation";

describe("parseBackendValidationMode", () => {
  it("defaults to off", () => {
    expect(parseBackendValidationMode(undefined)).toBe("off");
    expect(parseBackendValidationMode("0")).toBe("off");
    expect(parseBackendValidationMode("false")).toBe("off");
  });

  it("accepts preview aliases", () => {
    expect(parseBackendValidationMode("preview")).toBe("preview");
    expect(parseBackendValidationMode("1")).toBe("preview");
    expect(parseBackendValidationMode("true")).toBe("preview");
  });

  it("rejects unknown modes", () => {
    expect(() => parseBackendValidationMode("maybe")).toThrow(
      "Unsupported backend validation mode: maybe"
    );
  });
});

describe("resolveBackendValidationSettings", () => {
  it("rejects unsupported eval modes", () => {
    expect(() =>
      resolveBackendValidationSettings({
        evalMode: "app",
        requestedMode: "preview",
      })
    ).toThrow('Backend validation mode "preview" is only supported for flow and script evals');
  });
});
