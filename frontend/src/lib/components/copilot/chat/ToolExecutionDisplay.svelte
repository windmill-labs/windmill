<script lang="ts">
	import { Loader2, ChevronDown, ChevronRight, Copy, Check, XCircle, Play } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { ToolDisplayMessage } from './shared'
	
	interface Props {
		message: ToolDisplayMessage
	}
	
	let { 
		message,
	}: Props = $props()
	
	let isExpanded = $state(message.showDetails || (message.isLoading && message.needsConfirmation))
	let copiedParams = $state(false)
	let copiedResult = $state(false)

	$inspect(message)
	
	// Check if we have content to display
	const hasParameters = $derived(message.parameters !== undefined && Object.keys(message.parameters).length > 0)
	const hasResult = $derived(message.result !== undefined && message.result !== null)

	// Format JSON for display and parameters
	function formatJson(obj: any): string {
		try {
			// If it's already a string, try to parse and re-stringify for formatting
			if (typeof obj === 'string') {
				try {
					const parsed = JSON.parse(obj)
					return JSON.stringify(parsed, null, 2)
				} catch {
					// If it's not valid JSON, return as is
					return obj
				}
			}
			// Otherwise stringify the object
			return JSON.stringify(obj, null, 2)
		} catch {
			return String(obj)
		}
	}
	
	// Copy to clipboard
	async function copyToClipboard(text: string, type: 'params' | 'result') {
		try {
			await navigator.clipboard.writeText(text)
			if (type === 'params') {
				copiedParams = true
				setTimeout(() => copiedParams = false, 2000)
			} else {
				copiedResult = true
				setTimeout(() => copiedResult = false, 2000)
			}
		} catch (err) {
			console.error('Failed to copy:', err)
		}
	}
</script>

<div class="bg-surface border border-gray-200 dark:border-gray-700 rounded-md overflow-hidden font-mono text-xs">
	<!-- Collapsible Header -->
	<button 
		class="w-full p-3 bg-surface-secondary hover:bg-surface-hover transition-colors flex items-center justify-between text-left border-b border-gray-200 dark:border-gray-700"
		onclick={() => isExpanded = !isExpanded}
		disabled={!message.showDetails}
	>
		<div class="flex items-center gap-2 flex-1">
			{#if message.showDetails}
				{#if isExpanded}
					<ChevronDown class="w-3 h-3 text-secondary" />
				{:else}
					<ChevronRight class="w-3 h-3 text-secondary" />
				{/if}
			{/if}
			
			{#if message.isLoading}
				<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
			{:else if message.error}
				<span class="text-red-500">✗</span>
			{:else if !message.isLoading && !message.error}
				<span class="text-green-500">✓</span>
			{:else}
				<span class="text-tertiary">○</span>
			{/if}
			
			<span class="text-primary font-medium text-2xs">
				{message.content}
			</span>
		</div>
	</button>
	
	<!-- Expanded Content -->
	{#if isExpanded}
		<div class="p-3 bg-surface space-y-3">
				<!-- Parameters Section -->
				{#if hasParameters}
					<div class="space-y-2">
						<div class="flex items-center justify-between">
							<span class="text-secondary text-2xs font-semibold uppercase tracking-wide">
								Parameters:
							</span>
							<button 
								class="p-1 rounded hover:bg-surface-secondary text-tertiary hover:text-secondary transition-colors"
								onclick={() => copyToClipboard(formatJson(message.parameters), 'params')}
								title="Copy parameters"
							>
								{#if copiedParams}
									<Check class="w-3 h-3 text-green-500" />
								{:else}
									<Copy class="w-3 h-3" />
								{/if}
							</button>
						</div>
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 overflow-x-auto max-h-64 overflow-y-auto">
							<pre class="text-2xs text-primary whitespace-pre-wrap">{formatJson(message.parameters)}</pre>
						</div>
					</div>
				{/if}
				
				<!-- Result Section -->
				 {#if !message.needsConfirmation}
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-secondary text-2xs font-semibold uppercase tracking-wide">
							Result:
						</span>
						{#if hasResult && !message.error}
							<button 
								class="p-1 rounded hover:bg-surface-secondary text-tertiary hover:text-secondary transition-colors"
								onclick={() => copyToClipboard(formatJson(message.result), 'result')}
								title="Copy result"
							>
								{#if copiedResult}
									<Check class="w-3 h-3 text-green-500" />
								{:else}
									<Copy class="w-3 h-3" />
								{/if}
							</button>
						{/if}
					</div>
					
					{#if message.isLoading}
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 flex items-center gap-2 text-tertiary">
							<Loader2 class="w-3 h-3 animate-spin" />
							<span class="text-2xs">Executing...</span>
						</div>
					{:else if message.error}
						<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3 overflow-x-auto max-h-64 overflow-y-auto">
							<pre class="text-2xs text-red-700 dark:text-red-300 whitespace-pre-wrap">{message.error}</pre>
						</div>
					{:else if hasResult}
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 overflow-x-auto max-h-64 overflow-y-auto">
							<pre class="text-2xs text-primary whitespace-pre-wrap">{formatJson(message.result)}</pre>
						</div>
					{:else}
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 text-center">
							<span class="text-2xs text-tertiary">No result yet</span>
						</div>
					{/if}
				</div>
				{/if}

				<!-- Confirmation Footer -->
				{#if message.needsConfirmation}
					<div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700 flex flex-row items-center justify-end gap-2">
						<Button
							variant="border"
							color="red"
							size="xs"
							on:click={() => {
								if (message.tool_call_id) {
									aiChatManager.handleToolConfirmation(message.tool_call_id, false)
								}
							}}
							startIcon={{ icon: XCircle }}
						>
						</Button>
						<Button
							variant="border"
							color="blue"
							size="xs"
							on:click={() => {
								if (message.tool_call_id) {
									aiChatManager.handleToolConfirmation(message.tool_call_id, true)
								}
							}}
							startIcon={{ icon: Play }}
						>
							Run
						</Button>
					</div>
				{/if}
		</div>
	{/if}
</div>