import { describe, expect, test } from "bun:test";
import { parseS3Object } from "../s3Types";

describe("parseS3Object", () => {
  test("bare string is a key in the default storage", () => {
    expect(parseS3Object("dir/file.json")).toEqual({ s3: "dir/file.json" });
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

  test("malformed s3:// URI keeps the legacy empty-key fallback", () => {
    // Never store a literal "s3://…" key — an auto-generated key on write is
    // the historical behavior for unparseable URIs.
    expect(parseS3Object("s3://broken")).toEqual({ s3: "" });
  });

  test("empty string stays an empty key (auto-generated on write)", () => {
    expect(parseS3Object("")).toEqual({ s3: "" });
  });

  test("record form passes through", () => {
    expect(parseS3Object({ s3: "x", storage: "b" })).toEqual({
      s3: "x",
      storage: "b",
    });
  });
});
