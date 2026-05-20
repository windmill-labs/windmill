// Postgres wire-protocol helpers used by `wmill datatable serve`.
//
// pg-gateway handles the startup/auth handshake for us, but it leaves
// query-response framing (RowDescription / DataRow / CommandComplete /
// ReadyForQuery) to the caller. The functions below build those four
// messages from a `RawOutputEnvelope` returned by the backend.
//
// References:
//   https://www.postgresql.org/docs/current/protocol-message-formats.html

const TEXT_OID = 25;

export interface RawOutputColumn {
  name: string;
  oid: number;
  type_name: string;
}

export interface RawOutputEnvelope {
  columns: RawOutputColumn[];
  rows: (string | null)[][];
}

export interface ExecuteResponse {
  message: Uint8Array;
  nextRowOffset: number;
  suspended: boolean;
}

const encoder = new TextEncoder();

/**
 * Build a single backend message: one type byte, a 4-byte length (covering
 * the length field itself but not the type byte), then the payload.
 */
function buildMessage(type: number, payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(1 + 4 + payload.byteLength);
  out[0] = type;
  const len = 4 + payload.byteLength;
  out[1] = (len >>> 24) & 0xff;
  out[2] = (len >>> 16) & 0xff;
  out[3] = (len >>> 8) & 0xff;
  out[4] = len & 0xff;
  out.set(payload, 5);
  return out;
}

function writeInt16(buf: Uint8Array, offset: number, v: number): void {
  buf[offset] = (v >>> 8) & 0xff;
  buf[offset + 1] = v & 0xff;
}

function writeInt32(buf: Uint8Array, offset: number, v: number): void {
  buf[offset] = (v >>> 24) & 0xff;
  buf[offset + 1] = (v >>> 16) & 0xff;
  buf[offset + 2] = (v >>> 8) & 0xff;
  buf[offset + 3] = v & 0xff;
}

/**
 * RowDescription ('T'): for each column, emit
 *   name CSTRING, tableOid INT32, attnum INT16,
 *   typeOid INT32, typeSize INT16, typeMod INT32, formatCode INT16
 *
 * We don't know the underlying table/attnum (the value travels through JSON,
 * not directly from the prepared statement), so we send 0 — psql is fine with
 * that. Format code 0 means "text" — we'll send DataRow values as UTF-8 bytes.
 */
export function buildRowDescription(columns: RawOutputColumn[]): Uint8Array {
  const fields = columns.map((col) => {
    const nameBytes = encoder.encode(col.name);
    const buf = new Uint8Array(nameBytes.byteLength + 1 + 4 + 2 + 4 + 2 + 4 + 2);
    let off = 0;
    buf.set(nameBytes, off);
    off += nameBytes.byteLength;
    buf[off++] = 0; // CSTRING terminator
    writeInt32(buf, off, 0); off += 4; // tableOid
    writeInt16(buf, off, 0); off += 2; // attnum
    writeInt32(buf, off, col.oid || TEXT_OID); off += 4; // typeOid (fall back to text)
    writeInt16(buf, off, -1); off += 2; // typeSize (-1 = variable)
    writeInt32(buf, off, -1); off += 4; // atttypmod
    writeInt16(buf, off, 0); off += 2; // format = text
    return buf;
  });
  const totalLen = 2 + fields.reduce((s, f) => s + f.byteLength, 0);
  const payload = new Uint8Array(totalLen);
  writeInt16(payload, 0, columns.length);
  let off = 2;
  for (const f of fields) {
    payload.set(f, off);
    off += f.byteLength;
  }
  return buildMessage(0x54 /* 'T' */, payload);
}

/**
 * DataRow ('D'): INT16 column count, then for each column either
 *   INT32 -1   (NULL)
 * or
 *   INT32 length + UTF-8 bytes
 */
export function buildDataRow(row: (string | null)[]): Uint8Array {
  const encoded = row.map((v) => (v === null ? null : encoder.encode(v)));
  const total =
    2 + encoded.reduce((s, e) => s + 4 + (e === null ? 0 : e.byteLength), 0);
  const payload = new Uint8Array(total);
  writeInt16(payload, 0, row.length);
  let off = 2;
  for (const e of encoded) {
    if (e === null) {
      writeInt32(payload, off, -1);
      off += 4;
    } else {
      writeInt32(payload, off, e.byteLength);
      off += 4;
      payload.set(e, off);
      off += e.byteLength;
    }
  }
  return buildMessage(0x44 /* 'D' */, payload);
}

/**
 * CommandComplete ('C'): null-terminated tag like "SELECT 12".
 * We always send SELECT-tagged completions — psql doesn't validate the tag
 * against the actual statement kind, so SET/CREATE/etc. just show as "SELECT 0".
 */
export function buildCommandComplete(tag: string): Uint8Array {
  const tagBytes = encoder.encode(tag);
  const payload = new Uint8Array(tagBytes.byteLength + 1);
  payload.set(tagBytes, 0);
  payload[tagBytes.byteLength] = 0;
  return buildMessage(0x43 /* 'C' */, payload);
}

/** ReadyForQuery ('Z') with the transaction-status indicator byte. */
export function buildReadyForQuery(status: "I" | "T" | "E" = "I"): Uint8Array {
  const payload = new Uint8Array(1);
  payload[0] = status.charCodeAt(0);
  return buildMessage(0x5a /* 'Z' */, payload);
}

/** EmptyQueryResponse ('I') with no payload. */
export function buildEmptyQueryResponse(): Uint8Array {
  return buildMessage(0x49 /* 'I' */, new Uint8Array(0));
}

/** ParseComplete ('1') with no payload. */
export function buildParseComplete(): Uint8Array {
  return buildMessage(0x31 /* '1' */, new Uint8Array(0));
}

/** BindComplete ('2') with no payload. */
export function buildBindComplete(): Uint8Array {
  return buildMessage(0x32 /* '2' */, new Uint8Array(0));
}

/** CloseComplete ('3') with no payload. */
export function buildCloseComplete(): Uint8Array {
  return buildMessage(0x33 /* '3' */, new Uint8Array(0));
}

/** NoData ('n') with no payload. */
export function buildNoData(): Uint8Array {
  return buildMessage(0x6e /* 'n' */, new Uint8Array(0));
}

/** PortalSuspended ('s') with no payload. */
export function buildPortalSuspended(): Uint8Array {
  return buildMessage(0x73 /* 's' */, new Uint8Array(0));
}

/** ParameterDescription ('t'): INT16 count + one INT32 OID per parameter. */
export function buildParameterDescription(typeOids: number[]): Uint8Array {
  const payload = new Uint8Array(2 + typeOids.length * 4);
  writeInt16(payload, 0, typeOids.length);
  let off = 2;
  for (const oid of typeOids) {
    writeInt32(payload, off, oid);
    off += 4;
  }
  return buildMessage(0x74 /* 't' */, payload);
}

/**
 * Translate a `RawOutputEnvelope` into the byte stream a Postgres client
 * expects in response to a Query message: RowDescription, one DataRow per
 * row, CommandComplete, ReadyForQuery.
 *
 * If there are no columns AND no rows, we treat the statement as a no-op
 * (e.g. `SET client_encoding`) and emit only CommandComplete + ReadyForQuery,
 * skipping RowDescription. psql is fine with either shape.
 */
export function buildQueryResponse(envelope: RawOutputEnvelope): Uint8Array {
  const parts: Uint8Array[] = [];
  if (envelope.columns.length > 0) {
    parts.push(buildRowDescription(envelope.columns));
    for (const row of envelope.rows) {
      parts.push(buildDataRow(row));
    }
  }
  parts.push(buildCommandComplete(`SELECT ${envelope.rows.length}`));
  parts.push(buildReadyForQuery("I"));
  return concat(parts);
}

export function buildExecuteResponse(
  envelope: RawOutputEnvelope,
  opts?: {
    includeRowDescription?: boolean;
    maxRows?: number;
    rowOffset?: number;
  },
): ExecuteResponse {
  const includeRowDescription = opts?.includeRowDescription ?? true;
  const maxRows = opts?.maxRows ?? 0;
  const rowOffset = opts?.rowOffset ?? 0;
  const remainingRows = envelope.rows.slice(rowOffset);
  const suspended = maxRows > 0 && remainingRows.length > maxRows;
  const rowsToSend = suspended ? remainingRows.slice(0, maxRows) : remainingRows;

  const parts: Uint8Array[] = [];
  if (includeRowDescription && envelope.columns.length > 0) {
    parts.push(buildRowDescription(envelope.columns));
  }
  for (const row of rowsToSend) {
    parts.push(buildDataRow(row));
  }
  if (suspended) {
    parts.push(buildPortalSuspended());
  } else {
    parts.push(buildCommandComplete(`SELECT ${envelope.rows.length}`));
  }

  return {
    message: concat(parts),
    nextRowOffset: rowOffset + rowsToSend.length,
    suspended,
  };
}

export function concat(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((s, p) => s + p.byteLength, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const p of parts) {
    out.set(p, off);
    off += p.byteLength;
  }
  return out;
}
