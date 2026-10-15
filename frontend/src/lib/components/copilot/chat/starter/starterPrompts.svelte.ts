import { onboardingProfile, type StarterPrompt } from '$lib/onboardingProfile'
import { useReducedMotion } from '$lib/svelte5Utils.svelte'
import { userStore } from '$lib/stores'
import { fromStore } from 'svelte/store'
import { resource } from 'runed'
import { ScriptService } from '$lib/gen'

/** Example prompts: the short `label` is shown as a clickable tag under the composer,
 * the `prompt` is what rotates through the placeholder / gets dropped into the input. */
export const defaultStarterPrompts: StarterPrompt[] = [
	{
		label: 'Sync Salesforce',
		prompt: 'Sync new Salesforce leads into a postgres table every hour'
	},
	{
		label: 'Ban Discord users',
		prompt:
			'Build a workflow that triggers on a Discord message, checks for offensive language using an LLM, and possibly block them'
	},
	{
		label: 'Weekly Slack report',
		prompt: 'Generate a weekly sales report from postgres and post it to Slack every Monday'
	}
]

/** An operator's session can run what is deployed, read runs and call MCP servers, but
 * cannot author scripts or apps. It composes flows only where the workspace grants
 * `operator_settings.builder_flows`, so none of these asks it to build anything. */
export const operatorStarterPrompts: StarterPrompt[] = [
	{
		label: 'Run a flow',
		prompt: 'Find the flow that onboards a new employee and run it for Jane Doe, starting Monday'
	},
	{
		label: 'Rerun failed jobs',
		prompt: 'List the jobs that failed today, tell me why, and rerun the ones that timed out'
	},
	{
		label: 'Summarize GitHub issues',
		prompt: "Read this week's open GitHub issues and summarize the ones that mention billing"
	}
]

/** The stock examples for the reader's role, unless the invite that brought this person here
 * wrote prompts for them — those replace the set outright rather than joining it, since a
 * prompt written for someone's own stack next to "Ban Discord users" reads as the generic one. */
export class StarterPrompts {
	#invite = $state<StarterPrompt[]>()
	#user = fromStore(userStore)
	readonly list = $derived(
		this.#invite ?? (this.#user.current?.operator ? operatorStarterPrompts : defaultStarterPrompts)
	)
	#hasItems: { readonly current: boolean | undefined }

	/** `workspace`: where the session acts, which decides whether the chips are offered. */
	constructor(workspace: () => string | undefined) {
		void onboardingProfile().then((p) => {
			if (p?.starter_prompts?.length) this.#invite = p.starter_prompts
		})
		this.#hasItems = resource(workspace, async (ws) => (ws ? workspaceHasItems(ws) : undefined))
	}

	/** The example chips are for a workspace with nothing in it yet: next to someone's own
	 * scripts and flows, "Ban Discord users" is noise. Withheld until the workspace is known
	 * to be empty, so they never show and then vanish. The examples still rotate through the
	 * placeholder either way. */
	get chipsOffered(): boolean {
		return this.#hasItems.current === false
	}
}

// Workspaces known to hold something, for the page load. An empty answer is not kept: the
// next mount asks again, so the chips go once the user has created their first item.
const workspacesWithItems = new Set<string>()

async function workspaceHasItems(workspace: string): Promise<boolean> {
	if (workspacesWithItems.has(workspace)) return true
	try {
		// What Home's list counts: library scripts and the user's own drafts included.
		const res = await ScriptService.listRunnables({
			workspace,
			includeWithoutMain: true,
			includeDraftOnly: true,
			perPage: 1
		})
		const has = (res.items?.length ?? 0) > 0
		if (has) workspacesWithItems.add(workspace)
		return has
	} catch (e) {
		// Unknown reads as "has items": no chips is the quieter mistake.
		console.error(`Could not check whether workspace ${workspace} has items:`, e)
		return true
	}
}

/** What turns an empty chat into the block that starts a session. */
export type ChatStarter = {
	/** The page the block is on: an empty session centres it in the pane, Home does not. */
	where: 'home' | 'session'
	/** Rotated through the placeholder. */
	prompts: StarterPrompt[]
	/** Also offered as chips under the box. */
	chips: boolean
}

const CYCLE_MS = 7_000
/** Must match the `duration-*` class on the placeholder overlay: the swap happens once the
 * fade-out has finished, and Tailwind only emits classes it finds written out in full. */
export const PLACEHOLDER_FADE_CLASS = 'transition-opacity duration-[600ms] ease-in-out'
const FADE_MS = 600

/**
 * Rotates a placeholder through `prompts` every CYCLE_MS: fade out, swap, fade back in.
 * Only while `active` — otherwise it would loop forever driving an unrendered input — and
 * not under reduced motion, where the first prompt simply stays put. The index survives
 * `active` going false, so the rotation resumes from the prompt currently displayed.
 *
 * Shown as an overlay rather than the field's `placeholder`: WebKit and Gecko do not run
 * transitions on `::placeholder`, so the fade would be a hard cut there.
 */
export class RotatingPlaceholder {
	#index = $state(0)
	#prompts: () => string[]
	visible = $state(true)

	get text(): string {
		const list = this.#prompts()
		return list.length ? list[this.#index % list.length] : ''
	}

	constructor(prompts: () => string[], active: () => boolean) {
		this.#prompts = prompts
		const reducedMotion = useReducedMotion()
		$effect(() => {
			if (!active() || reducedMotion.val) return
			let timer: ReturnType<typeof setTimeout>

			const next = () => {
				this.visible = false
				timer = setTimeout(() => {
					this.#index = (this.#index + 1) % Math.max(prompts().length, 1)
					this.visible = true
					timer = setTimeout(next, CYCLE_MS)
				}, FADE_MS)
			}

			timer = setTimeout(next, CYCLE_MS)
			return () => {
				clearTimeout(timer)
				// Torn down mid-fade, the input would otherwise remount with an invisible placeholder.
				this.visible = true
			}
		})
	}
}
