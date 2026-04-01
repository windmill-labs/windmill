#!/usr/bin/env npx tsx
/**
 * Regenerate cli/wmill.schema.json from CONFIG_REFERENCE.
 *
 * Run after adding or modifying config options in src/commands/init/template.ts:
 *   npx tsx generate-schema.ts
 */
import { writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { generateJsonSchema } from "./src/commands/init/template.ts";

const dir = dirname(fileURLToPath(import.meta.url));
const out = join(dir, "wmill.schema.json");
writeFileSync(out, JSON.stringify(generateJsonSchema(), null, 2) + "\n");
console.log(`Wrote ${out}`);
