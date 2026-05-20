import { expect, test } from "bun:test";
import {
  buildBindComplete,
  buildCloseComplete,
  buildExecuteResponse,
  buildCommandComplete,
  buildDataRow,
  buildEmptyQueryResponse,
  buildNoData,
  buildParameterDescription,
  buildParseComplete,
  buildPortalSuspended,
  buildQueryResponse,
  buildReadyForQuery,
  buildRowDescription,
  type RawOutputEnvelope,
} from "../src/commands/datatable/pg_wire.ts";

// Postgres wire-protocol byte-level expectations. Every backend message is:
//   1 byte type | int32 length-including-itself | payload

test("buildReadyForQuery: 'Z' + len=5 + 'I'", () => {
  const buf = buildReadyForQuery("I");
  expect(Array.from(buf)).toEqual([
    0x5a, // 'Z'
    0, 0, 0, 5, // length = 4 (length field) + 1 (status)
    0x49, // 'I'
  ]);
});

test("buildCommandComplete: 'C' + tag + null terminator", () => {
  const buf = buildCommandComplete("SELECT 0");
  // 'C' + len(4) + 'SELECT 0'(8) + '\0'(1) → 13 bytes total, length field = 13
  expect(buf[0]).toBe(0x43);
  expect(buf.byteLength).toBe(14);
  expect(buf[buf.byteLength - 1]).toBe(0);
});

test("buildEmptyQueryResponse: 'I' + len=4", () => {
  const buf = buildEmptyQueryResponse();
  expect(Array.from(buf)).toEqual([0x49, 0, 0, 0, 4]);
});

test("buildParseComplete / BindComplete / CloseComplete / NoData / PortalSuspended: empty payload messages", () => {
  expect(Array.from(buildParseComplete())).toEqual([0x31, 0, 0, 0, 4]);
  expect(Array.from(buildBindComplete())).toEqual([0x32, 0, 0, 0, 4]);
  expect(Array.from(buildCloseComplete())).toEqual([0x33, 0, 0, 0, 4]);
  expect(Array.from(buildNoData())).toEqual([0x6e, 0, 0, 0, 4]);
  expect(Array.from(buildPortalSuspended())).toEqual([0x73, 0, 0, 0, 4]);
});

test("buildParameterDescription: count + OIDs", () => {
  const buf = buildParameterDescription([23, 25]);
  expect(buf[0]).toBe(0x74); // 't'
  expect((buf[5] << 8) | buf[6]).toBe(2);
  const firstOid =
    (buf[7] << 24) |
    (buf[8] << 16) |
    (buf[9] << 8) |
    buf[10];
  const secondOid =
    (buf[11] << 24) |
    (buf[12] << 16) |
    (buf[13] << 8) |
    buf[14];
  expect(firstOid).toBe(23);
  expect(secondOid).toBe(25);
});

test("buildRowDescription: encodes one column with given OID", () => {
  const buf = buildRowDescription([{ name: "id", oid: 23, type_name: "int4" }]);
  // 'T'(1) + len(4) + field_count(2) + name(2 + null=3) + tableOid(4) + attnum(2)
  // + typeOid(4) + typeSize(2) + atttypmod(4) + format(2) = 1 + 4 + 2 + 3 + 4 + 2 + 4 + 2 + 4 + 2 = 28
  expect(buf[0]).toBe(0x54); // 'T'
  expect(buf.byteLength).toBe(28);
  // field count INT16 starts at offset 5 (after type byte + length field)
  expect((buf[5] << 8) | buf[6]).toBe(1);
  // typeOid INT32 is at the end of the column header — at offset 28 - 2 - 4 - 2 - 4 = 16
  const typeOidOffset = 28 - 2 /* format */ - 4 /* typmod */ - 2 /* typesize */ - 4;
  const typeOid =
    (buf[typeOidOffset] << 24) |
    (buf[typeOidOffset + 1] << 16) |
    (buf[typeOidOffset + 2] << 8) |
    buf[typeOidOffset + 3];
  expect(typeOid).toBe(23);
});

test("buildDataRow: NULL cell is encoded as int32 -1, length 0 payload", () => {
  const buf = buildDataRow([null]);
  // 'D'(1) + len(4) + col_count(2) + col_length(4 = -1) = 11 bytes
  expect(Array.from(buf)).toEqual([
    0x44, // 'D'
    0, 0, 0, 10, // length field = 4 + 2 + 4
    0, 1, // col_count = 1
    0xff, 0xff, 0xff, 0xff, // -1 (NULL)
  ]);
});

test("buildDataRow: text cell carries UTF-8 bytes prefixed by length", () => {
  const buf = buildDataRow(["hi"]);
  expect(Array.from(buf)).toEqual([
    0x44,
    0, 0, 0, 12, // length = 4 + 2 + 4 + 2
    0, 1, // col count
    0, 0, 0, 2, // string length
    0x68, 0x69, // "hi"
  ]);
});

test("buildDataRow: mixed text + NULL in column order", () => {
  const buf = buildDataRow(["a", null, "bb"]);
  // 'D'(1) + len(4) + col_count(2) + (4 + 1) + 4 + (4 + 2) = 22
  expect(buf.byteLength).toBe(22);
  // col count
  expect((buf[5] << 8) | buf[6]).toBe(3);
  // First cell length=1, byte 'a' at offset 11
  expect(buf[11]).toBe(0x61);
  // Second cell is NULL: bytes 12-15 should be 0xff
  expect(buf[12]).toBe(0xff);
  expect(buf[15]).toBe(0xff);
  // Third cell length=2 at offset 16, bytes 'b','b' at 20-21
  expect(buf[20]).toBe(0x62);
  expect(buf[21]).toBe(0x62);
});

test("buildQueryResponse: empty envelope emits only CommandComplete + ReadyForQuery", () => {
  const envelope: RawOutputEnvelope = { columns: [], rows: [] };
  const buf = buildQueryResponse(envelope);
  // CommandComplete "SELECT 0" = 14 bytes, ReadyForQuery = 6 bytes
  expect(buf.byteLength).toBe(14 + 6);
  expect(buf[0]).toBe(0x43); // 'C'
  expect(buf[14]).toBe(0x5a); // 'Z'
});

test("buildQueryResponse: one column + one row emits T, D, C, Z in that order", () => {
  const envelope: RawOutputEnvelope = {
    columns: [{ name: "n", oid: 23, type_name: "int4" }],
    rows: [["42"]],
  };
  const buf = buildQueryResponse(envelope);
  const tagBytes: number[] = [];
  let off = 0;
  while (off < buf.byteLength) {
    tagBytes.push(buf[off]);
    const len = (buf[off + 1] << 24) | (buf[off + 2] << 16) | (buf[off + 3] << 8) | buf[off + 4];
    off += 1 + len;
  }
  expect(tagBytes).toEqual([0x54 /* T */, 0x44 /* D */, 0x43 /* C */, 0x5a /* Z */]);
});

test("buildExecuteResponse: emits T, D, C without ReadyForQuery", () => {
  const envelope: RawOutputEnvelope = {
    columns: [{ name: "n", oid: 23, type_name: "int4" }],
    rows: [["42"]],
  };
  const response = buildExecuteResponse(envelope);
  const tagBytes: number[] = [];
  let off = 0;
  while (off < response.message.byteLength) {
    tagBytes.push(response.message[off]);
    const len =
      (response.message[off + 1] << 24) |
      (response.message[off + 2] << 16) |
      (response.message[off + 3] << 8) |
      response.message[off + 4];
    off += 1 + len;
  }
  expect(response.suspended).toBe(false);
  expect(response.nextRowOffset).toBe(1);
  expect(tagBytes).toEqual([0x54 /* T */, 0x44 /* D */, 0x43 /* C */]);
});

test("buildExecuteResponse: maxRows suspends portal and omits CommandComplete", () => {
  const envelope: RawOutputEnvelope = {
    columns: [{ name: "n", oid: 23, type_name: "int4" }],
    rows: [["1"], ["2"]],
  };
  const response = buildExecuteResponse(envelope, { maxRows: 1 });
  const tagBytes: number[] = [];
  let off = 0;
  while (off < response.message.byteLength) {
    tagBytes.push(response.message[off]);
    const len =
      (response.message[off + 1] << 24) |
      (response.message[off + 2] << 16) |
      (response.message[off + 3] << 8) |
      response.message[off + 4];
    off += 1 + len;
  }
  expect(response.suspended).toBe(true);
  expect(response.nextRowOffset).toBe(1);
  expect(tagBytes).toEqual([0x54 /* T */, 0x44 /* D */, 0x73 /* s */]);
});
