import { expect, test } from "bun:test";
// app.ts first: it is the entry the CLI loads lint.ts through, and loading
// lint.ts ahead of it trips their import cycle.
import "../src/commands/app/app.ts";
import { viewerIdentityInputWarnings } from "../src/commands/app/lint.ts";

const sql = (content: string, fields?: Record<string, any>) => ({
  type: "inline",
  inlineScript: { language: "duckdb", content },
  ...(fields ? { fields } : {}),
});

test("app lint warns on a SQL viewer-identity argument the server does not fill", async () => {
  const content =
    "-- $username (text)\n-- $limit (int)\nSELECT * FROM orders WHERE owner = $username LIMIT $limit";

  const warnings = await viewerIdentityInputWarnings({
    open: sql(content),
    filled: sql(content, { username: { ctx: "username" } }),
  });

  expect(warnings).toHaveLength(1);
  expect(warnings[0]).toContain("Runnable 'open': argument 'username'");
});
