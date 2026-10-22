import { onboardingProfile, type StarterPrompt } from '$lib/onboardingProfile'
import { roleCapabilities } from '$lib/stores'
import { fromStore } from 'svelte/store'
import { resource } from 'runed'
import { ScriptService } from '$lib/gen'

/** A prompt offered under the composer: `label` is the pill, `prompt` what it puts in the
 * box. `use` looks at, asks about or runs what exists; `build` creates or changes things. */
export type ComposerSuggestion = StarterPrompt & { mode: 'use' | 'build' }

const USE_EXAMPLES: ComposerSuggestion[] = [
	{
		mode: 'use',
		label: 'What failed today?',
		prompt: 'Which runs failed in this workspace today, and why?'
	},
	{
		mode: 'use',
		label: 'Which schedules run the most?',
		prompt:
			'Which schedules ran the most today, and does any of them run more often than it should?'
	}
]

const BUILD_EXAMPLES: ComposerSuggestion[] = [
	{
		mode: 'build',
		label: 'Sync Salesforce',
		prompt: 'Sync new Salesforce leads into a postgres table every hour'
	},
	{
		mode: 'build',
		label: 'Weekly Slack report',
		prompt: 'Generate a weekly sales report from postgres and post it to Slack every Monday'
	},
	{
		mode: 'build',
		label: 'Ban Discord users',
		prompt:
			'Build a workflow that triggers on a Discord message, checks for offensive language using an LLM, and possibly block them'
	}
]

/** A session for a role that cannot code can run what is deployed, read runs and call MCP
 * servers, but cannot author scripts, so none of these asks it to build anything. */
const OPERATOR_EXAMPLES: ComposerSuggestion[] = [
	USE_EXAMPLES[0],
	{
		mode: 'use',
		label: 'Rerun failed jobs',
		prompt: 'List the jobs that failed today, tell me why, and rerun the ones that timed out'
	},
	{
		mode: 'use',
		label: 'Run a flow',
		prompt: 'Find the flow that onboards a new employee and run it for Jane Doe, starting Monday'
	},
	{
		mode: 'use',
		label: 'Summarize GitHub issues',
		prompt: "Read this week's open GitHub issues and summarize the ones that mention billing"
	}
]

/** From this many items on, Home's composer rests collapsed: below it the workspace is new
 * enough that the composer is the page's main way in. */
const FEW_ITEMS = 10

/** The suggestions for the reader's role and the workspace the session acts on, and whether
 * that workspace is still small. An invite that wrote prompts for this person replaces the
 * examples outright: a prompt written for someone's own stack next to "Ban Discord users"
 * reads as the generic one. */
export class StarterPrompts {
	#invite = $state<StarterPrompt[]>()
	#capabilities = fromStore(roleCapabilities)
	#workspace: () => string | undefined
	#itemCount: { readonly current: number | undefined }

	/** `workspace`: where the session acts. */
	constructor(workspace: () => string | undefined) {
		this.#workspace = workspace
		void onboardingProfile().then((p) => {
			if (p?.starter_prompts?.length) this.#invite = p.starter_prompts
		})
		this.#itemCount = resource(workspace, async (ws) => (ws ? workspaceItemCount(ws) : undefined))
	}

	/** At most `FEW_ITEMS`; undefined until the workspace has answered. A count already read
	 * this page load stands in while it is asked again, so a remount does not wait on it. */
	get #count(): number | undefined {
		const ws = this.#workspace()
		return this.#itemCount.current ?? (ws ? itemCounts.get(ws) : undefined)
	}

	/** Up to four: two Use then two Build. An empty workspace has nothing to ask about yet. */
	readonly suggestions: ComposerSuggestion[] = $derived.by(() => {
		if (this.#invite) return this.#invite.map((p) => ({ ...p, mode: 'build' as const }))
		if (!this.#capabilities.current.code) return OPERATOR_EXAMPLES
		if (this.#count === 0) return BUILD_EXAMPLES
		return [...USE_EXAMPLES, ...BUILD_EXAMPLES.slice(0, 2)]
	})

	/** Whether the workspace holds fewer than `FEW_ITEMS`; undefined until known. */
	get fewItems(): boolean | undefined {
		const count = this.#count
		return count === undefined ? undefined : count < FEW_ITEMS
	}
}

const itemCounts = new Map<string, number>()
const itemCountRequests = new Map<string, Promise<number>>()

/** Starts the count for a page about to mount the starter; the starter then takes this
 * request rather than sending its own. */
export function prefetchItemCount(workspace: string): void {
	void workspaceItemCount(workspace)
}

function workspaceItemCount(workspace: string): Promise<number> {
	let request = itemCountRequests.get(workspace)
	if (!request) {
		request = fetchItemCount(workspace).finally(() => itemCountRequests.delete(workspace))
		itemCountRequests.set(workspace, request)
	}
	return request
}

async function fetchItemCount(workspace: string): Promise<number> {
	let count = FEW_ITEMS
	try {
		// What Home's list counts: library scripts and the user's own drafts included.
		const res = await ScriptService.listRunnables({
			workspace,
			includeWithoutMain: true,
			includeDraftOnly: true,
			perPage: FEW_ITEMS
		})
		count = res.items?.length ?? 0
	} catch (e) {
		// Unknown reads as a full workspace: a collapsed composer is the quieter mistake.
		console.error(`Could not count the items of workspace ${workspace}:`, e)
	}
	itemCounts.set(workspace, count)
	return count
}

/** What turns an empty chat into Home's block that starts a session, collapsed to one line at
 * rest unless `fewItems`. */
export type ChatStarter = { suggestions: ComposerSuggestion[]; fewItems: boolean }
