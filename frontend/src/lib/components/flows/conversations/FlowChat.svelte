<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { chatIdentity, type FlowChatProps } from './flowChatProps'
	import FlowChatPanel from './FlowChatPanel.svelte'

	let props: FlowChatProps = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')

	/**
	 * A chat holds one flow's conversations, the turn running in one of them, and the rows
	 * that turn is writing. Pointing the same chat at another flow would leave every one of
	 * those in flight — an upload, a launch, a poll, a list request — landing in the chat
	 * that replaced it, so the panel is replaced wholesale instead and nothing outlives it.
	 *
	 * The workspace is the one the surface operates on, not the navigated one: an embedded
	 * editor acts on a session's fork while `workspaceStore` stays where the reader left it.
	 */
	const chatKey = $derived(
		chatIdentity(flowEditorContext?.opWorkspace?.() ?? $workspaceStore, props)
	)
</script>

{#key chatKey}
	<FlowChatPanel {...props} />
{/key}
