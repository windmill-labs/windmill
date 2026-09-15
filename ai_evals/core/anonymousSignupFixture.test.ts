import { describe, expect, it } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// The disclosure case asserts on what the assistant said and on the draft it produced, so
// nothing in it ever looks at the seeded app. A wrong idiom here is invisible to the eval and
// gets copied: the model reads this runnable before writing its sibling, and reproduced a
// non-executing `await sql` from an earlier version of this fixture in every run.
const fixture = JSON.parse(
  readFileSync(
    join(
      import.meta.dir,
      "../fixtures/frontend/global/initial/anonymous_signup_app.json",
    ),
    "utf8",
  ),
);

const app = fixture.workspace.apps[0];
const runnables = app.value.runnables;
const signups =
  fixture.workspace.datatables[0].schemas.public.signups;

describe("anonymous_signup_app fixture", () => {
  it("runs its SQL instead of only building it", () => {
    for (const [key, runnable] of Object.entries<any>(runnables)) {
      const content = runnable.inlineScript.content;
      // A statement is built by the tag and only runs on .execute()/.fetch*(); `await` on the
      // un-terminated statement resolves to the statement itself and writes nothing.
      expect(content, `${key} must terminate its statement`).toMatch(
        /`\s*\.(execute|fetch|fetchOne|fetchOneScalar)\(/,
      );
      expect(content, `${key} must not interpolate into SQL`).not.toMatch(/'\$\{/);
    }
  });

  it("declares the inputs its generated binding is typed from", () => {
    for (const [key, runnable] of Object.entries<any>(runnables)) {
      const declared = Object.keys(
        runnable.inlineScript.schema?.properties ?? {},
      );
      const args =
        runnable.inlineScript.content.match(/export async function main\(([^)]*)\)/)?.[1] ?? "";
      const actual = args
        .split(",")
        .map((a: string) => a.split(":")[0].trim())
        .filter(Boolean);
      expect(declared.sort(), `${key} schema must match main()`).toEqual(actual.sort());
    }
  });

  it("calls backend keys that exist", () => {
    const called = [
      ...app.value.files["/App.tsx"].matchAll(/backend\.(\w+)\(/g),
    ].map((m) => m[1]);
    expect(called.length).toBeGreaterThan(0);
    for (const key of called) {
      expect(runnables, `/App.tsx calls backend.${key}`).toHaveProperty(key);
    }
  });

  it("seeds the table its runnables query", () => {
    expect(Array.isArray(signups.columns)).toBe(false);
    for (const row of signups.rows ?? []) {
      for (const column of Object.keys(row)) {
        expect(signups.columns, `seeded row column ${column}`).toHaveProperty(column);
      }
    }
    for (const runnable of Object.values<any>(runnables)) {
      for (const [, table] of runnable.inlineScript.content.matchAll(
        /(?:INTO|FROM|UPDATE)\s+public\.(\w+)/gi,
      )) {
        expect(table, "queried table must be seeded").toBe("signups");
      }
    }
  });
});
