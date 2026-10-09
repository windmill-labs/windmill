import { onboardingProfile, type StarterPrompt } from '$lib/onboardingProfile'
import { useReducedMotion } from '$lib/svelte5Utils.svelte'

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

/** The stock examples, unless the invite that brought this person here wrote prompts for
 * them — those replace the set outright rather than joining it, since a prompt written
 * for someone's own stack next to "Ban Discord users" reads as the generic one. */
export class StarterPrompts {
	list = $state(defaultStarterPrompts)

	constructor() {
		void onboardingProfile().then((p) => {
			if (p?.starter_prompts?.length) this.list = p.starter_prompts
		})
	}
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
