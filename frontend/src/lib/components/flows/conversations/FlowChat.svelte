<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createFlowChatManager } from './FlowChatManager.svelte'
	import FlowConversationsSidebar from './FlowConversationsSidebar.svelte'
	import FlowChatInterface from './FlowChatInterface.svelte'
	import { untrack } from 'svelte'

	interface Props {
		onRunFlow: (userMessage: string, conversationId: string) => Promise<string | undefined>
		useStreaming?: boolean
		deploymentInProgress?: boolean
		path: string
		hideSidebar?: boolean
	}

	let {
		onRunFlow,
		deploymentInProgress = false,
		useStreaming = false,
		path,
		hideSidebar = false
	}: Props = $props()

	const manager = createFlowChatManager()

	// Initialize manager when component mounts
	$effect(() => {
		if ($workspaceStore) {
			manager.initialize(onRunFlow, path, useStreaming)
		}

		return () => {
			manager.cleanup()
		}
	})

	// Initialize InfiniteList when component mounts or flowPath changes
	$effect(() => {
		if ($workspaceStore && path && manager.conversationListComponent) {
			untrack(() => {
				manager.setupInfiniteList()
			})
		}
	})
</script>

<div class="flex border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden flex-1">
	{#if !hideSidebar}
		<FlowConversationsSidebar {manager} />
	{/if}
	<FlowChatInterface {manager} {deploymentInProgress} />
</div>
