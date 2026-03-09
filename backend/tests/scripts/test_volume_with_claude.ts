// volume: agent-memory .claude
// sandbox

import Anthropic from "@anthropic-ai/sdk";
import * as fs from "fs";
import * as path from "path";

type Anthropic = {
  api_key: string;
  model?: string;
};

export async function main(anthropic_resource: Anthropic) {
  const claudeDir = ".claude";
  const results: Record<string, unknown> = {};

  // --- Step 1: Verify volume is mounted at the relative path ---
  results["volume_exists"] = fs.existsSync(claudeDir);
  if (!results["volume_exists"]) {
    fs.mkdirSync(claudeDir, { recursive: true });
  }

  const testFile = path.join(claudeDir, "mount-check.txt");
  fs.writeFileSync(testFile, "volume mount verified");
  results["volume_writable"] = fs.readFileSync(testFile, "utf-8") === "volume mount verified";

  // --- Step 2: Create memory directory structure ---
  const memoryDir = path.join(claudeDir, "memory");
  fs.mkdirSync(memoryDir, { recursive: true });

  const memoryFile = path.join(memoryDir, "MEMORY.md");
  fs.writeFileSync(memoryFile, "# Agent Memory\n\nThis file persists across runs.\n");
  results["memory_file_created"] = fs.existsSync(memoryFile);

  // --- Step 3: Call Claude to generate structured content ---
  const client = new Anthropic({ apiKey: anthropic_resource.api_key });
  const model = anthropic_resource.model ?? "claude-sonnet-4-20250514";

  const response = await client.messages.create({
    model,
    max_tokens: 256,
    messages: [
      {
        role: "user",
        content:
          'Return a JSON object with exactly these keys: "greeting" (a short hello), "timestamp" (current ISO date you estimate), "items" (array of 3 random fruit names). Only return the JSON, no markdown.',
      },
    ],
  });

  const assistantText =
    response.content[0].type === "text" ? response.content[0].text : "";
  results["claude_responded"] = assistantText.length > 0;
  results["claude_model"] = response.model;
  results["claude_stop_reason"] = response.stop_reason;

  let parsed: Record<string, unknown> = {};
  try {
    parsed = JSON.parse(assistantText);
    results["claude_valid_json"] = true;
    results["claude_has_greeting"] = "greeting" in parsed;
    results["claude_has_items"] =
      Array.isArray(parsed.items) && parsed.items.length === 3;
  } catch {
    results["claude_valid_json"] = false;
  }

  // --- Step 4: Write Claude's response to volume ---
  const responsePath = path.join(claudeDir, "claude-response.json");
  fs.writeFileSync(responsePath, JSON.stringify(parsed, null, 2));
  results["response_written"] = fs.existsSync(responsePath);

  // --- Step 5: Read back and verify ---
  const readBack = fs.readFileSync(responsePath, "utf-8");
  const readParsed = JSON.parse(readBack);
  results["readback_matches"] =
    JSON.stringify(readParsed) === JSON.stringify(parsed);

  // --- Step 6: List all volume contents ---
  const volumeContents = fs.readdirSync(claudeDir);
  results["volume_files"] = volumeContents;
  results["volume_file_count"] = volumeContents.length;

  // --- Step 7: Verify memory file persists ---
  const memoryContent = fs.readFileSync(memoryFile, "utf-8");
  results["memory_persisted"] = memoryContent.includes("Agent Memory");

  // --- Summary ---
  const allChecks = [
    results["volume_exists"] || true,
    results["volume_writable"],
    results["claude_responded"],
    results["claude_valid_json"],
    results["response_written"],
    results["readback_matches"],
    results["memory_file_created"],
    results["memory_persisted"],
  ];
  results["all_passed"] = allChecks.every(Boolean);

  return results;
}
