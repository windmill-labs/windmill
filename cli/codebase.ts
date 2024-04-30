import { Codebase } from "./conf.ts";

export async function buildCodebase(codebase: Codebase, language: "bun") {
  if (language != "bun") {
    throw new Error("Only bun is supported for codebases currently");
  }
  if (codebase.kind != "tar_v1") {
    throw new Error("Only tar_v1 is supported for codebases currently");
  }
  console.log(
    `Building codebase at ${codebase.relative_path} with build command ${codebase.buildcmd}`
  );
}
