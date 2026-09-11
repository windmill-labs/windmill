import type { AIProvider } from '$lib/gen'
import type { Item } from '$lib/utils'
import { REASONING_OFF } from './reasoningRegistry'

/**
 * The contract between a chat and its model button.
 *
 * One component renders this menu for every chat — the copilot's own session chat and
 * the flow chat — so the component knows only about rows, choices and a reasoning
 * ladder. What a row means (a workspace AI resource, a prompt to edit, a reading
 * preference) is the caller's business, and each caller derives its own config: a fixed
 * one for the session chat, one derived from the flow's exposed inputs for flow chat.
 */

export type ModelChoice = {
	/** Stable across rebuilds of the config; used as the `{#each}` key. */
	key: string
	label: string
	/** Muted trailing text, e.g. the provider a resource speaks. */
	hint?: string
	selected: boolean
	onSelect: () => void
}

export type ChoiceSection = {
	/** Section heading, e.g. 'Provider' or 'Model'. */
	label: string
	options: ModelChoice[]
	/** Fetched lists render one consistent loading line instead of the options. */
	loading?: boolean
	/** Shown when the list is empty and settled. */
	emptyMessage?: string
	maxHeight?: string
}

export type ChatModelSettingsConfig = {
	/** The trigger's main text: the chosen model, or an invitation to choose one. */
	label: string
	title?: string
	/** Trailing pill on the trigger, e.g. the free-tier grant this chat is spending. */
	badge?: { text: string; warn?: boolean }
	/** Nothing here is editable — the trigger still names the model, but no menu opens. */
	readOnly?: boolean
	readOnlyReason?: string
	/**
	 * Rows above and below the choice sections. Given the menu's own `close` because a
	 * row that opens a modal must close the menu first, while a row that toggles a
	 * preference must not.
	 */
	topItems?: (close: () => void) => Item[]
	sections?: ChoiceSection[]
	bottomItems?: (close: () => void) => Item[]
	/**
	 * The thinking slider. The ladder is derived from the provider and model here rather
	 * than by each caller, so a new provider's effort levels reach every chat at once.
	 * `value` is the raw stored effort (undefined meaning the model's default), and
	 * `offToken` the token this caller stores for "off" — the copilot keeps its own
	 * sentinel and translates when it calls the provider, an agent writes the
	 * provider-native token straight into its step.
	 */
	reasoning?: ChatModelSettingsReasoning
}

export type ChatModelSettingsReasoning = {
	provider: AIProvider
	model: string
	value: string | undefined
	offToken: string | undefined
	/**
	 * What an unset value means on the wire. The copilot fills one in before calling the
	 * provider, so unset really runs at the default effort and the button says so. An agent
	 * step omits the field entirely, so unset means whatever the provider does by itself —
	 * naming a level there would state something the run does not do.
	 */
	sendsDefaultWhenUnset: boolean
	onSelect: (token: string) => void
}

/** What an unset agent effort reads as: the provider decides, and we do not know what. */
export const REASONING_PROVIDER_DEFAULT = 'default'

/**
 * The effort to keep when the model changes, or nothing where the new model has no such
 * level. Dropped rather than carried because a model that cannot think at that level either
 * rejects the request or quietly runs at another one, and the button would name a level the
 * run never used. Off survives only onto a model that can truly disable.
 *
 * A model the registry has no rules for keeps whatever it had: dropping on `supported:
 * false` would discard a real setting on the strength of never having heard of the
 * provider, and nothing would draw a control to put it back.
 */
export function carriedReasoning(
	current: string | undefined,
	offToken: string | undefined,
	capability: { levels: string[]; canDisable: boolean; known: boolean }
): string | undefined {
	if (current === undefined || current === '') return undefined
	if (!capability.known) return current
	if (offToken !== undefined && current === offToken) {
		return capability.canDisable ? current : undefined
	}
	return capability.levels.includes(current) ? current : undefined
}

/**
 * What the menu shows for the reasoning ladder: the stops the slider offers, the one it
 * sits on, and the suffix on the trigger.
 *
 * Pure and here rather than in the component because these three have to agree — a stop
 * the slider renders as `off` must not read as the provider's own `none` on the button —
 * and because the rules are provider-shaped enough to be worth testing directly.
 */
export function reasoningDisplay(
	reasoning: ChatModelSettingsReasoning | undefined,
	capability: { supported: boolean; levels: string[]; canDisable: boolean },
	effective: string | undefined
): {
	stops: string[]
	currentStop: string
	/** Trigger suffix, or undefined when there is nothing truthful to say. */
	label: string | undefined
} {
	if (!reasoning || !capability.supported) {
		return { stops: [], currentStop: '', label: undefined }
	}
	// An off position only where the model can truly disable, else the provider would
	// coerce it to the lowest level; then the provider-native levels.
	const offToken = capability.canDisable ? reasoning.offToken : undefined
	const stops = [...(offToken !== undefined ? [offToken] : []), ...capability.levels]
	// An agent whose model disables by omission stores the empty string, which the run
	// treats as no effort at all — so an unset value already sits on that stop.
	const isOff = offToken !== undefined && (reasoning.value ?? '') === offToken
	if (isOff) {
		// The off token is provider-native and can read as anything ('none', 'disabled');
		// on the button and on the slider it always reads as off.
		return { stops, currentStop: offToken as string, label: REASONING_OFF }
	}
	if (reasoning.value === undefined || reasoning.value === '') {
		// Where the provider takes an explicit disable, unset is a third state — the
		// provider's own level, above off — that the ladder has no position for. The
		// button still names it, and every stop the ladder does offer stays reachable.
		return reasoning.sendsDefaultWhenUnset
			? { stops, currentStop: effective ?? '', label: effective ?? REASONING_OFF }
			: { stops, currentStop: '', label: REASONING_PROVIDER_DEFAULT }
	}
	return { stops, currentStop: reasoning.value, label: reasoning.value }
}
