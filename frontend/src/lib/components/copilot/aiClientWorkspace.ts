// The workspace the AI clients go through (their proxy URL names it). Kept apart from
// `./lib`, which holds the provider SDKs: what switches workspace runs in the app shell, and
// the clients are built on their first use rather than on every switch.
let current: string | undefined
let generation = 0

export function setAiClientWorkspace(workspace: string): void {
	current = workspace
	generation++
}

export function aiClientWorkspace(): { workspace: string | undefined; generation: number } {
	return { workspace: current, generation }
}
