<script lang="ts" module>
	import type { SessionTarget } from './sessionState.svelte'

	// What an editor hands over for "Open in AI session": the session target it
	// maps to, the workspace it lives in, and a persist hook run before routing
	// so the session preview opens the item exactly as currently edited.
	export type OpenInSessionSource = {
		/** The item the preview opens on. Surfaces that aren't editable items pass
		 * `page` instead; exactly one of the two is set. */
		target?: SessionTarget
		/** Base-prefixed href of a workspace page the preview opens as a tab (Runs,
		 * a trigger list). Resolved on click, not at render: a page whose filters
		 * live in shallow-routed query params never reflects them in `page.url`, so
		 * only `window.location` read at that moment matches what the user sees. */
		page?: () => string | undefined
		workspaceId?: string
		beforeOpen?: () => void | Promise<void>
		/** Where inside the item the preview should open (a flow's `selected`
		 * step). Steers the editor only — tab identity is (kind, path). */
		previewParams?: Record<string, string>
	}
</script>

<script lang="ts">
	import { getContext, type ComponentProps, type Snippet } from 'svelte'
	import { BROWSER } from 'esm-env'
	import AIButton from '$lib/components/copilot/chat/AIButton.svelte'
	import { AIBtnClasses } from '$lib/components/copilot/chat/AIButtonStyle'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { userStore } from '$lib/stores'
	import { openEditorInSession, openPageInSession } from './sessionSwitch.svelte'

	let {
		source,
		btnClasses,
		btnProps,
		fallback
	}: {
		/** Undefined (e.g. an item without a path yet) renders the fallback. */
		source?: OpenInSessionSource
		btnClasses?: string
		/** Button styling overrides for hosts with their own conventions (an
		 * editor toolbar). */
		btnProps?: ComponentProps<typeof AIButton>['btnProps']
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
	// The sessions page refuses operators, so an entry point on a page they can
	// reach (Runs, the trigger lists) would only route them into that refusal.
	const show = $derived(
		!inSessionPanel &&
			!!(source?.target || source?.page) &&
			!$userStore?.operator &&
			isGlobalAiEnabled()
	)

	// Not $state: only read inside open() as a re-entrancy latch, never rendered.
	let opening = false
	async function open() {
		if (opening || !source) return
		opening = true
		try {
			await source.beforeOpen?.()
			if (source.target) {
				await openEditorInSession(source.target, source.workspaceId, source.previewParams)
			} else {
				const href = source.page?.()
				if (href) await openPageInSession(href, source.workspaceId)
			}
		} finally {
			opening = false
		}
	}
</script>

{#if show}
	<AIButton togglePanel={open} btnClasses={btnClasses ?? AIBtnClasses('default')} {btnProps} />
{:else if !inSessionPanel}
	{@render fallback?.()}
{/if}
