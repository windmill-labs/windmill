import { expect, test } from "bun:test";
import {
  devUploadKey,
  parseArgBinding,
  parseS3Uri,
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

test("parseArgBinding: JSON values parse, non-JSON falls back to the raw string", () => {
  // a bare date is not valid JSON → string
  expect(parseArgBinding("daily_report:partition=2026-07-02")).toEqual({
    scriptTok: "daily_report",
    param: "partition",
    value: "2026-07-02",
  });
  // JSON scalars/objects keep their type
  expect(parseArgBinding("stats:limit=10")).toEqual({
    scriptTok: "stats",
    param: "limit",
    value: 10,
  });
  expect(parseArgBinding("stats:enabled=true")).toEqual({
    scriptTok: "stats",
    param: "enabled",
    value: true,
  });
  expect(parseArgBinding('f/pd/stats:opts={"a":1}')).toEqual({
    scriptTok: "f/pd/stats",
    param: "opts",
    value: { a: 1 },
  });
  // quoting forces a string even for number-looking values
  expect(parseArgBinding('stats:code="42"')).toEqual({
    scriptTok: "stats",
    param: "code",
    value: "42",
  });
  // split on the FIRST `=` — values may contain `=`
  expect(parseArgBinding("stats:expr=a=b")).toEqual({
    scriptTok: "stats",
    param: "expr",
    value: "a=b",
  });
});

test("parseArgBinding: :param is required and malformed specs throw", () => {
  expect(() => parseArgBinding("stats=10")).toThrow(); // no :param
  expect(() => parseArgBinding("stats:limit")).toThrow(); // no `=`
  expect(() => parseArgBinding(":limit=10")).toThrow(); // empty script
  expect(() => parseArgBinding("stats:=10")).toThrow(); // empty param
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

test("s3ObjectParams: matches SQL-dialect resource-S3Object casing too", () => {
  // TS/python parsers emit `resource-s3_object`; the SQL dialects (duckdb
  // `-- $file (s3object)`) emit `resource-S3Object` — both must bind.
  const schema = {
    properties: {
      file: { type: "object", format: "resource-S3Object" },
      other: { type: "object", format: "resource-s3" },
    },
  };
  expect(s3ObjectParams(schema)).toEqual(["file"]);
});

test("devUploadKey: key scoped by script path + param so basenames don't clobber", () => {
  expect(devUploadKey("f/analytics/ingest", "file", "./fixtures/events.csv")).toBe(
    "wmilldev/pipeline/f/analytics/ingest/file/events.csv",
  );
  // same basename, different script/param → distinct keys
  expect(devUploadKey("f/analytics/other", "file", "/abs/events.csv")).toBe(
    "wmilldev/pipeline/f/analytics/other/file/events.csv",
  );
  expect(devUploadKey("f/analytics/ingest", "backup", "/abs/events.csv")).toBe(
    "wmilldev/pipeline/f/analytics/ingest/backup/events.csv",
  );
});

test("s3Arg: shapes the S3Object run-arg", () => {
  expect(s3Arg("file", { s3: "wmilldev/pipeline/analytics/events.csv" })).toEqual({
    file: { s3: "wmilldev/pipeline/analytics/events.csv" },
  });
  expect(s3Arg("file", { s3: "k.csv", storage: "secondary" })).toEqual({
    file: { s3: "k.csv", storage: "secondary" },
  });
});

test("parseS3Uri: canonical `s3://<storage>/<key>` (authority is the named storage)", () => {
  // named storage — the authority is the storage, the rest is the key
  expect(parseS3Uri("s3://secondary/k.csv")).toEqual({ s3: "k.csv", storage: "secondary" });
  // empty authority (`s3:///key`) ⇒ default storage, incl. nested keys
  expect(parseS3Uri("s3:///events.csv")).toEqual({ s3: "events.csv" });
  expect(parseS3Uri("s3:///raw/2026/events.csv")).toEqual({ s3: "raw/2026/events.csv" });
  // named storage with a nested key
  expect(parseS3Uri("s3://secondary/raw/2026/events.csv")).toEqual({
    s3: "raw/2026/events.csv",
    storage: "secondary",
  });
  // no `/` after the scheme → no authority; whole rest is the default-storage key
  expect(parseS3Uri("s3://events.csv")).toEqual({ s3: "events.csv" });
});
