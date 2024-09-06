import type { CommandResult, SudoOptions } from "../cli.ts";

export type { CommandResult };
export * from "../cli/common.ts";

export async function run(cmd: string, args: string[]): Promise<CommandResult> {
    void cmd, args;
    throw new Error("Unsupported runtime");
}

export async function powershell(script: string): Promise<CommandResult> {
    void script;
    throw new Error("Unsupported runtime");
}

export async function sudo(
    cmd: string,
    args: string[],
    options: SudoOptions = {}
): Promise<CommandResult> {
    void cmd, args, options;
    throw new Error("Unsupported runtime");
}

export async function which(cmd: string): Promise<string | null> {
    void cmd;
    throw new Error("Unsupported runtime");
}

export async function edit(filename: string): Promise<void> {
    void filename;
    throw new Error("Unsupported runtime");
}
