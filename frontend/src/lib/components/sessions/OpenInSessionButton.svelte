<script lang="ts" module>
	import type { SessionTarget } from './sessionState.svelte'

	// What an editor hands over for "Open in AI session": the session target it
	// maps to, the workspace it lives in, and a persist hook run before routing
	// so the session preview opens the item exactly as currently edited.
	type OpenInSessionCommon = {
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

	// A destination is either an editable item or a page, never both and never
	// neither — the union is what makes that a compile error rather than a button
	// that silently does nothing.
	export type OpenInSessionSource = OpenInSessionCommon &
		(
			| { target: SessionTarget; page?: never }
			/** Base-prefixed href of a workspace page the preview opens as a tab (Runs,
			 * a trigger list). Resolved on click, not at render: a page whose filters
			 * live in shallow-routed query params never reflects them in `page.url`, so
			 * only `window.location` read at that moment matches what the user sees. */
			| { page: () => string | undefined; target?: never }
		)
</script>

<script lang="ts">
	import { getContext, type ComponentProps, type Snippet } from 'svelte'
	import { BROWSER } from 'esm-env'
	import AIButton from '$lib/components/copilot/chat/AIButton.svelte'
	import { AIBtnClasses } from '$lib/components/copilot/chat/AIButtonStyle'
	import { prefersSessionHandoff } from '$lib/components/copilot/chat/global/gate'
	import { copilotInfo } from '$lib/aiStore'
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
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
	// prefersSessionHandoff carries the operator clause: the sessions page refuses
	// them, so an entry point on a page they can reach (Runs, the trigger lists)
	// would only route them into that refusal.
	const show = $derived(
		!inSessionPanel &&
			!!(source?.target || source?.page) &&
			prefersSessionHandoff($userStore?.operator)
	)

	// Not $state: only read inside open() as a re-entrancy latch, never rendered.
	let opening = false
	async function open() {
		if (opening || !source) return
		opening = true
		try {
			// `beforeOpen` (run inside openSourceInSession) persists what is on screen
			// and throws when it could not, so a failure has to stay on this page and
			// say so — the session would otherwise open on an older draft than the
			// editor the user is looking at.
			await openSourceInSession(source)
		} catch (e) {
			sendUserToast(e instanceof Error ? e.message : String(e), true)
		} finally {
			opening = false
		}
	}
</script>

{#if $copilotInfo.workspaceDisabled}
	<!-- The workspace hid the assistant: neither the hand-off nor the docked-chat
	     fallback has anywhere to lead, so no host renders an AI button at all. -->
{:else if show}
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
