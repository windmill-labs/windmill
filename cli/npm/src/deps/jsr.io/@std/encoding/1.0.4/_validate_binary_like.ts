// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

const encoder = new TextEncoder();

function getTypeName(value: unknown): string {
  const type = typeof value;
  if (type !== "object") {
    return type;
  } else if (value === null) {
    return "null";
  } else {
    return value?.constructor?.name ?? "object";
  }
}

export function validateBinaryLike(source: unknown): Uint8Array {
  if (typeof source === "string") {
    return encoder.encode(source);
  } else if (source instanceof Uint8Array) {
    return source;
  } else if (source instanceof ArrayBuffer) {
    return new Uint8Array(source);
  }
  throw new TypeError(
    `Cannot validate the input as it must be a Uint8Array, a string, or an ArrayBuffer: received a value of the type ${
      getTypeName(source)
    }`,
  );
}
