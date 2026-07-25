<script lang="ts" module>
	import type { SessionTarget } from './sessionState.svelte'

	// What an editor hands over for "Open in AI session": the session target it
	// maps to, the workspace it lives in, and a persist hook run before routing
	// so the session preview opens the item exactly as currently edited.
	export type OpenInSessionSource = {
		target: SessionTarget
		workspaceId?: string
		beforeOpen?: () => void | Promise<void>
	}
</script>

<script lang="ts">
	import { getContext, type Snippet } from 'svelte'
	import { BROWSER } from 'esm-env'
	import AIButton from '$lib/components/copilot/chat/AIButton.svelte'
	import { AIBtnClasses } from '$lib/components/copilot/chat/AIButtonStyle'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { openEditorInSession } from './sessionSwitch.svelte'

	let {
		source,
		btnClasses,
		fallback
	}: {
		/** Undefined (e.g. an item without a path yet) renders the fallback. */
		source?: OpenInSessionSource
		btnClasses?: string
		/** Rendered instead when the user opted out of the sessions beta
		 * (typically the editor's inline-chat toggle). Never rendered inside
		 * the session panel. */
		fallback?: Snippet
	} = $props()

	// Inside the session panel the chat is already on screen — a second entry
	// point would nest sessions, so render nothing at all there. In-realm
	// preview editors sit under the 'aiChatManager' context (set by
	// SessionEditorTarget / the session wrapper); iframe preview tabs are not
	// the top window.
	const inSessionPanel = !!getContext('aiChatManager') || (BROWSER && window.self !== window.top)
	const show = $derived(!inSessionPanel && !!source && isGlobalAiEnabled())

	// Not $state: only read inside open() as a re-entrancy latch, never rendered.
	let opening = false
	async function open() {
		if (opening || !source) return
		opening = true
		try {
			await source.beforeOpen?.()
			await openEditorInSession(source.target, source.workspaceId)
		} finally {
			opening = false
		}
	}
</script>

{#if show}
	<AIButton togglePanel={open} btnClasses={btnClasses ?? AIBtnClasses('default')} />
{:else if !inSessionPanel}
	{@render fallback?.()}
{/if}
