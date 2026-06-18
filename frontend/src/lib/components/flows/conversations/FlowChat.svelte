<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createFlowChatManager } from './FlowChatManager.svelte'
	import FlowConversationsSidebar from './FlowConversationsSidebar.svelte'
	import FlowChatInterface from './FlowChatInterface.svelte'
	import { untrack } from 'svelte'

	interface Props {
		onRunFlow: (
			userMessage: string,
			conversationId: string,
			additionalInputs?: Record<string, any>
		) => Promise<string | undefined>
		useStreaming?: boolean
		deploymentInProgress?: boolean
		path: string
		hideSidebar?: boolean
		inputSchema?: Record<string, any>
	}

	let {
		onRunFlow,
		deploymentInProgress = false,
		useStreaming = false,
		path,
		hideSidebar = false,
		inputSchema = undefined,
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

	// Derive additional inputs schema (excluding user_message) for chat mode
	const additionalInputsSchema = $derived.by(() => {
		const props = inputSchema?.properties ?? {}
		const filtered = Object.fromEntries(
			Object.entries(props).filter(([k]) => k !== 'user_message')
		)
		if (Object.keys(filtered).length === 0) return undefined
		const required = inputSchema?.required
		const requiredArray: string[] = Array.isArray(required) ? required : []
		return {
			...inputSchema,
			properties: filtered,
			required: requiredArray.filter((k: string) => k !== 'user_message')
		}
	})
</script>

<div class="flex border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden flex-1">
	{#if !hideSidebar}
		<FlowConversationsSidebar {manager} />
	{/if}
	<FlowChatInterface {manager} {deploymentInProgress} {additionalInputsSchema} {path} />
</div>
