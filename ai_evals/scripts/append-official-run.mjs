#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { appendOfficialRun, DEFAULT_HISTORY_DIR } from "../history/writer.mjs";

async function main() {
  const { inputPath, historyDir } = parseArgs(process.argv.slice(2));
  const input = JSON.parse(await readFile(path.resolve(inputPath), "utf8"));
  const result = await appendOfficialRun(input, {
    historyDir
  });

  process.stdout.write(JSON.stringify(result, null, 2) + "\n");
}

function parseArgs(argv) {
  let inputPath;
  let historyDir = DEFAULT_HISTORY_DIR;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--input") {
      inputPath = argv[index + 1];
      index += 1;
      continue;
    }
    if (arg === "--history-dir") {
      historyDir = argv[index + 1];
      index += 1;
      continue;
    }
    if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    }
    throw new Error(`Unknown argument: ${arg}`);
  }

  if (!inputPath) {
    throw new Error("Missing required --input /path/to/run.json argument");
  }

  return { inputPath, historyDir };
}

function printHelp() {
  process.stdout.write(
    [
      "Usage:",
      "  node ai_evals/scripts/append-official-run.mjs --input /path/to/run.json",
      "",
      "Options:",
      "  --history-dir /path/to/history   Override the history directory",
      "  --help                           Show this message"
    ].join("\n") + "\n"
  );
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});
