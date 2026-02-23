import { Command } from "@cliffy/command";
import { ZshCompletionsGenerator } from "@cliffy/command/completions/_zsh_completions_generator";
const { default: command } = await import("./src/main.ts");

// Generate completions manually
const output = ZshCompletionsGenerator.generate("wmill", command);
console.error(`output length: ${output.length}`);

// Write using process.stdout.write with callback
process.stdout.write(output + "\n", () => {
  console.error("write callback called");
  process.stdin.destroy();
});
