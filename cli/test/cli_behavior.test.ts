import { expect, test, describe } from "bun:test";
import { parseCliBehavior } from "../src/core/conf.ts";

describe("parseCliBehavior", () => {
  test("parses v1 to 1", () => {
    expect(parseCliBehavior("v1")).toBe(1);
  });

  test("parses v2 to 2", () => {
    expect(parseCliBehavior("v2")).toBe(2);
  });

  test("parses v10 to 10", () => {
    expect(parseCliBehavior("v10")).toBe(10);
  });

  test("returns 0 for undefined", () => {
    expect(parseCliBehavior(undefined)).toBe(0);
  });

  test("returns 0 for empty string", () => {
    expect(parseCliBehavior("")).toBe(0);
  });

  test("returns 0 for invalid format", () => {
    expect(parseCliBehavior("0.1")).toBe(0);
    expect(parseCliBehavior("1")).toBe(0);
    expect(parseCliBehavior("version1")).toBe(0);
    expect(parseCliBehavior("V1")).toBe(0);
  });

  test("version comparisons work correctly", () => {
    expect(parseCliBehavior("v1") >= 1).toBe(true);
    expect(parseCliBehavior("v2") >= 1).toBe(true);
    expect(parseCliBehavior(undefined) >= 1).toBe(false);
    expect(parseCliBehavior("v1") >= 2).toBe(false);
    expect(parseCliBehavior("v2") >= 2).toBe(true);
  });
});
