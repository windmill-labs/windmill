import { pack } from "tar-stream";

export interface TarEntry {
  name: string;
  content: Buffer | Uint8Array | string;
}

export function createTarBlob(entries: TarEntry[]): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const p = pack();
    const chunks: Uint8Array[] = [];

    p.on("data", (chunk: Buffer) => chunks.push(new Uint8Array(chunk)));
    p.on("end", () => resolve(new Blob(chunks as BlobPart[])));
    p.on("error", reject);

    for (const entry of entries) {
      p.entry({ name: entry.name }, Buffer.from(entry.content));
    }
    p.finalize();
  });
}
