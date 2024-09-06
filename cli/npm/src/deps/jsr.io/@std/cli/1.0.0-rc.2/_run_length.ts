// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

export function runLengthEncode(arr: number[]) {
  const data: number[] = [];
  const runLengths: number[] = [];

  let prev: symbol | number = Symbol("none");

  for (const x of arr) {
    if (x === prev) {
      ++runLengths[runLengths.length - 1];
    } else {
      prev = x;
      data.push(x);
      runLengths.push(1);
    }
  }

  for (const r of runLengths) {
    if (r >= 0x100) {
      throw new Error(`Run length too long: ${r}`);
    }
  }

  return {
    d: btoa(String.fromCharCode(...data)),
    r: btoa(String.fromCharCode(...runLengths)),
  };
}

export function runLengthDecode({ d, r }: { d: string; r: string }) {
  const data = atob(d);
  const runLengths = atob(r);
  let out = "";

  for (const [i, ch] of [...runLengths].entries()) {
    out += data[i]!.repeat(ch.codePointAt(0)!);
  }

  return Uint8Array.from([...out].map((x) => x.codePointAt(0)!));
}
