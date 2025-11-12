<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createFlowChatManager } from './NewFlowChatManager.svelte'
	import NewFlowConversationsSidebar from './NewFlowConversationsSidebar.svelte'
	import NewFlowChatInterface from './NewFlowChatInterface.svelte'
	import { untrack } from 'svelte'

	interface Props {
		onRunFlow: (userMessage: string, conversationId: string) => Promise<string | undefined>
		useStreaming?: boolean
		deploymentInProgress?: boolean
		path?: string
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
			manager.initialize({
				onRunFlow,
				useStreaming,
				path
			})
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
		<NewFlowConversationsSidebar {manager} />
	{/if}
	<NewFlowChatInterface {manager} {deploymentInProgress} />
</div>
