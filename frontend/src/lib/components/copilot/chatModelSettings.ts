import type { AIProvider } from '$lib/gen'
import type { Item } from '$lib/utils'

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
	reasoning?: {
		provider: AIProvider
		model: string
		value: string | undefined
		offToken: string | undefined
		onSelect: (token: string) => void
	}
}
