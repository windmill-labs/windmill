import { describe, expect, test } from "bun:test";
import { parseS3Object } from "../s3Types";

describe("parseS3Object", () => {
  test("bare string throws with the s3:/// hint", () => {
    // A bare key is not accepted: older clients silently degraded it to an
    // empty key (auto-generated key on write), hiding typos.
    expect(() => parseS3Object("dir/file.json" as any)).toThrow(
      /s3:\/\/\/dir\/file\.json/
    );
  });

  test("triple-slash URI targets the default storage", () => {
    expect(parseS3Object("s3:///dir/file.json")).toEqual({
      storage: undefined,
      s3: "dir/file.json",
    });
  });

  test("full URI splits storage and key", () => {
    expect(parseS3Object("s3://bucket/dir/f")).toEqual({
      storage: "bucket",
      s3: "dir/f",
    });
  });

  test("malformed s3:// URI throws", () => {
    // `s3://x` has no key part — fail loudly instead of silently misplacing
    // the object.
    expect(() => parseS3Object("s3://broken" as any)).toThrow(
      /Invalid s3 object/
    );
  });

  test("empty string throws (omit the object for an auto-generated key)", () => {
    expect(() => parseS3Object("" as any)).toThrow(/Invalid s3 object/);
  });

  test("record form passes through", () => {
    expect(parseS3Object({ s3: "x", storage: "b" })).toEqual({
      s3: "x",
      storage: "b",
    });
  });
});
