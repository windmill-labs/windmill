<script lang="ts" module>
	import type { SessionTarget } from './sessionState.svelte'

	// What an editor hands over for "Open in AI session": the session target it
	// maps to, the workspace it lives in, and a persist hook run before routing
	// so the session preview opens the item exactly as currently edited.
	export type OpenInSessionSource = {
		target: SessionTarget
		workspaceId?: string
		beforeOpen?: () => void | Promise<void>
		/** Where inside the item the preview should open (a flow's `selected`
		 * step). Steers the editor only — tab identity is (kind, path). */
		previewParams?: Record<string, string>
		/** Pre-fills the new session's composer. Entry points that carry an intent
		 * (fix this error, run this item) hand it over as text rather than driving
		 * a chat the caller cannot see. */
		seedPrompt?: string
		/** Send `seedPrompt` on arrival rather than parking it in the composer.
		 * For clicks that already stated the intent; leave it off where the prompt
		 * is a proposal the user should read first. */
		autoSend?: boolean
	}
</script>

<script lang="ts">
	import { getContext, type ComponentProps, type Snippet } from 'svelte'
	import { BROWSER } from 'esm-env'
	import AIButton from '$lib/components/copilot/chat/AIButton.svelte'
	import { AIBtnClasses } from '$lib/components/copilot/chat/AIButtonStyle'
	import { prefersSessionHandoff } from '$lib/components/copilot/chat/global/gate'
	import { userStore } from '$lib/stores'
	import { openSourceInSession } from './sessionSwitch.svelte'

	let {
		source,
		btnClasses,
		btnProps,
		label,
		tooltip,
		fallback
	}: {
		/** Undefined (e.g. an item without a path yet) renders the fallback. */
		source?: OpenInSessionSource
		btnClasses?: string
		/** Button styling overrides for hosts with their own conventions (an
		 * editor toolbar). */
		btnProps?: ComponentProps<typeof AIButton>['btnProps']
		/** Names the action this replaced, for hosts whose button carried its own
		 * label ("AI Fix"). Defaults to AIButton's generic "Open in AI session". */
		label?: string
		/** Hover text. Pass it whenever `label` is set: a renamed button no longer
		 * says that clicking it leaves for a session. */
		tooltip?: string
		/** Rendered instead when the caller keeps a docked chat to drive — an
		 * opted-out user or an operator (typically the editor's inline-chat
		 * toggle). Never rendered inside the session panel. */
		fallback?: Snippet
	} = $props()

	// Inside the session panel the chat is already on screen — a second entry
	// point would nest sessions, so render nothing at all there. In-realm
	// preview editors sit under the 'aiChatManager' context (set by
	// SessionEditorTarget / the session wrapper); iframe preview tabs are not
	// the top window.
	const inSessionPanel = !!getContext('aiChatManager') || (BROWSER && window.self !== window.top)
	const show = $derived(!inSessionPanel && !!source && prefersSessionHandoff($userStore?.operator))

	// Not $state: only read inside open() as a re-entrancy latch, never rendered.
	let opening = false
	async function open() {
		if (opening || !source) return
		opening = true
		try {
			await openSourceInSession(source)
		} finally {
			opening = false
		}
	}
</script>

{#if show}
	<AIButton
		togglePanel={open}
		btnClasses={btnClasses ?? AIBtnClasses('default')}
		{btnProps}
		{label}
		{tooltip}
	/>
{:else if !inSessionPanel}
	{@render fallback?.()}
{/if}
