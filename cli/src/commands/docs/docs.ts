import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { GlobalOptions } from "../../types.ts";
import { getHeaders } from "../../utils/utils.ts";
import { detectAuthGatewayChallenge } from "../../utils/http_guards.ts";

interface DocsSearchResult {
  url: string;
  title: string;
  score: number;
  snippets: string[];
}

interface DocsSearchResponse {
  text: string;
  results: DocsSearchResult[];
}

async function docs(
  opts: GlobalOptions & { json?: boolean },
  query: string
) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);

  // The backend self-hosts the docs corpus and does the search, so this works
  // against any instance (no windmill.dev egress required).
  const url = `${workspace.remote}api/docs/search?query=${encodeURIComponent(query)}`;

  console.log(colors.bold(`\nSearching Windmill docs...\n`));

  const extraHeaders = getHeaders();
  let res: Response;
  try {
    res = await fetch(url, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${workspace.token}`,
        ...extraHeaders,
      },
    });
  } catch (e) {
    throw new Error(`Network error connecting to ${workspace.remote}: ${e}`);
  }

  await detectAuthGatewayChallenge(res, url);

  if (!res.ok) {
    throw new Error(
      `Documentation search failed: ${res.status} ${res.statusText}\n${await res.text()}`
    );
  }

  const data = (await res.json()) as DocsSearchResponse;
  const items = data.results ?? [];

  if (opts.json) {
    console.log(JSON.stringify(items, null, 2));
    return;
  }

  if (items.length === 0) {
    log.info("No documentation found for this query.");
    return;
  }

  for (const item of items) {
    console.log(colors.bold(colors.cyan(`📄 ${item.title}`)));
    if (item.url) {
      console.log(`   ${colors.underline(item.url)}`);
    }
    for (const snippet of item.snippets ?? []) {
      console.log(`   ${snippet}`);
    }
    console.log();
  }
}

const command = new Command()
  .name("docs")
  .description("Search Windmill documentation.")
  .arguments("<query:string>")
  .option("--json", "Output results as JSON.")
  .action(docs as any);

export default command;
