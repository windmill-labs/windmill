import { expect, test } from "bun:test";
import {
  devUploadKey,
  parseUploadBinding,
  s3Arg,
  s3ObjectParams,
} from "../src/commands/pipeline/pipelineUpload.ts";

test("parseUploadBinding: bare script + local source infers the param later", () => {
  expect(parseUploadBinding("ingest=./fixtures/events.csv")).toEqual({
    scriptTok: "ingest",
    source: "./fixtures/events.csv",
  });
});

test("parseUploadBinding: explicit :param on the left", () => {
  expect(parseUploadBinding("f/analytics/ingest:file=./events.csv")).toEqual({
    scriptTok: "f/analytics/ingest",
    param: "file",
    source: "./events.csv",
  });
});

test("parseUploadBinding: s3:// source — its `:` is right of the first `=`, so it stays intact", () => {
  expect(parseUploadBinding("ingest=s3://bucket/2026/events.csv")).toEqual({
    scriptTok: "ingest",
    source: "s3://bucket/2026/events.csv",
  });
  expect(parseUploadBinding("ingest:file=s3://bucket/k")).toEqual({
    scriptTok: "ingest",
    param: "file",
    source: "s3://bucket/k",
  });
});

test("parseUploadBinding: malformed specs throw", () => {
  expect(() => parseUploadBinding("ingest")).toThrow(); // no `=`
  expect(() => parseUploadBinding("=./x.csv")).toThrow(); // no script
  expect(() => parseUploadBinding("ingest=")).toThrow(); // no source
  expect(() => parseUploadBinding("ingest:=./x.csv")).toThrow(); // empty param
  expect(() => parseUploadBinding(":file=./x.csv")).toThrow(); // empty script
});

test("s3ObjectParams: only resource-s3_object properties, in declaration order", () => {
  const schema = {
    properties: {
      note: { type: "string" },
      file: { type: "object", format: "resource-s3_object" },
      db: { type: "object", format: "resource-postgresql" },
      backup: { type: "object", format: "resource-s3_object" },
    },
  };
  expect(s3ObjectParams(schema)).toEqual(["file", "backup"]);
  expect(s3ObjectParams({})).toEqual([]);
  expect(s3ObjectParams(undefined)).toEqual([]);
});

test("devUploadKey: deterministic folder-scoped key from the source basename", () => {
  expect(devUploadKey("analytics", "./fixtures/events.csv")).toBe(
    "wmilldev/pipeline/analytics/events.csv",
  );
  expect(devUploadKey("analytics", "/abs/path/to/data.parquet")).toBe(
    "wmilldev/pipeline/analytics/data.parquet",
  );
});

test("s3Arg: shapes the S3Object run-arg", () => {
  expect(s3Arg("file", "wmilldev/pipeline/analytics/events.csv")).toEqual({
    file: { s3: "wmilldev/pipeline/analytics/events.csv" },
  });
});
