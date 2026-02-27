import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { GlobalOptions } from "../../types.ts";

interface DocContentItem {
  title: string;
  url: string;
  source?: {
    content?: Array<{ text: string }>;
  };
}

interface InkeepResponse {
  choices?: Array<{
    message?: {
      content?: string;
    };
  }>;
}

interface ParsedContent {
  content?: DocContentItem[];
}

async function docs(
  opts: GlobalOptions & { json?: boolean },
  query: string
) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);

  const url = `${workspace.remote}api/inkeep`;

  console.log(colors.bold(`\nSearching Windmill docs...\n`));

  let res: Response;
  try {
    res = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${workspace.token}`,
      },
      body: JSON.stringify({ query }),
    });
  } catch (e) {
    throw new Error(`Network error connecting to ${workspace.remote}: ${e}`);
  }

  if (res.status === 403) {
    log.info(
      "Windmill documentation search is an Enterprise Edition feature. Please upgrade to use this command."
    );
    return;
  }

  if (!res.ok) {
    throw new Error(
      `Documentation search failed: ${res.status} ${res.statusText}\n${await res.text()}`
    );
  }

  const data = (await res.json()) as InkeepResponse;
  const raw = data.choices?.[0]?.message?.content;

  if (!raw) {
    log.info("No documentation found for this query.");
    return;
  }

  let parsed: ParsedContent;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("Failed to parse documentation response.");
  }

  const items = parsed.content ?? [];

  if (items.length === 0) {
    log.info("No documentation found for this query.");
    return;
  }

  if (opts.json) {
    console.log(JSON.stringify(items, null, 2));
    return;
  }

  for (const item of items) {
    console.log(colors.bold(colors.cyan(`📄 ${item.title}`)));
    if (item.url) {
      console.log(`   ${colors.underline(item.url)}`);
    }
    const text = item.source?.content?.[0]?.text;
    if (text) {
      const snippet = text.length > 500 ? text.slice(0, 500) + "..." : text;
      console.log(`   ${snippet}`);
    }
    console.log();
  }
}

const command = new Command()
  .name("docs")
  .description("Search Windmill documentation. Requires Enterprise Edition.")
  .arguments("<query:string>")
  .option("--json", "Output results as JSON.")
  .action(docs as any);

export default command;
