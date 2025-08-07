<script lang="ts">
	import { Loader2, ChevronDown, ChevronRight, Copy, Check } from 'lucide-svelte'
	
	interface Props {
		toolName: string
		description?: string
		parameters?: Record<string, any>
		result?: any
		isLoading?: boolean
		error?: string
		collapsed?: boolean
		showDetails?: boolean
	}
	
	let { 
		toolName, 
		parameters = {},
		result = undefined,
		isLoading = false,
		error = undefined,
		collapsed = false,
		showDetails = false
	}: Props = $props()
	
	let isExpanded = $state(!collapsed)
	let copiedParams = $state(false)
	let copiedResult = $state(false)
	
	// Format the tool name for display
	const displayName = $derived(
		toolName.startsWith('api_') 
			? toolName.substring(4)
				.replace(/([A-Z])/g, ' $1')
				.replace(/_/g, ' ')
				.trim()
				.toLowerCase()
				.replace(/^\w/, c => c.toUpperCase())
			: toolName
	)
	
	// Check if we have content to display
	const hasParameters = $derived(Object.keys(parameters).length > 0)
	const hasResult = $derived(result !== undefined && result !== null)
	
	// Format JSON for display
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
	>
		<div class="flex items-center gap-2 flex-1">
			{#if isExpanded}
				<ChevronDown class="w-3 h-3 text-secondary" />
			{:else}
				<ChevronRight class="w-3 h-3 text-secondary" />
			{/if}
			
			{#if isLoading}
				<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
			{:else if error}
				<span class="text-red-500">✗</span>
			{:else if hasResult}
				<span class="text-green-500">✓</span>
			{:else}
				<span class="text-tertiary">○</span>
			{/if}
			
			<span class="text-primary font-medium text-2xs">
				Called {displayName}
			</span>
		</div>
		
		<!-- Status Badge -->
		<div class="flex items-center gap-2">
			{#if isLoading}
				<span class="px-2 py-1 rounded text-2xs font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 border border-blue-200 dark:border-blue-700">
					Running
				</span>
			{:else if error}
				<span class="px-2 py-1 rounded text-2xs font-medium bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 border border-red-200 dark:border-red-700">
					Failed
				</span>
			{:else if hasResult}
				<span class="px-2 py-1 rounded text-2xs font-medium bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 border border-green-200 dark:border-green-700">
					Completed
				</span>
			{/if}
		</div>
	</button>
	
	<!-- Expanded Content -->
	{#if isExpanded}
		<div class="p-3 bg-surface space-y-3">
			{#if showDetails}
				<!-- Parameters Section -->
				{#if hasParameters}
					<div class="space-y-2">
						<div class="flex items-center justify-between">
							<span class="text-secondary text-2xs font-semibold uppercase tracking-wide">
								Parameters:
							</span>
							<button 
								class="p-1 rounded hover:bg-surface-secondary text-tertiary hover:text-secondary transition-colors"
								onclick={() => copyToClipboard(formatJson(parameters), 'params')}
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
							<pre class="text-2xs text-primary whitespace-pre-wrap">{formatJson(parameters)}</pre>
						</div>
					</div>
				{/if}
				
				<!-- Result Section -->
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-secondary text-2xs font-semibold uppercase tracking-wide">
							Result:
						</span>
						{#if hasResult && !error}
							<button 
								class="p-1 rounded hover:bg-surface-secondary text-tertiary hover:text-secondary transition-colors"
								onclick={() => copyToClipboard(formatJson(result), 'result')}
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
					
					{#if isLoading}
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 flex items-center gap-2 text-tertiary">
							<Loader2 class="w-3 h-3 animate-spin" />
							<span class="text-2xs">Executing...</span>
						</div>
					{:else if error}
						<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3 overflow-x-auto max-h-64 overflow-y-auto">
							<pre class="text-2xs text-red-700 dark:text-red-300 whitespace-pre-wrap">{error}</pre>
						</div>
					{:else if hasResult}
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 overflow-x-auto max-h-64 overflow-y-auto">
							<pre class="text-2xs text-primary whitespace-pre-wrap">{formatJson(result)}</pre>
						</div>
					{:else}
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 text-center">
							<span class="text-2xs text-tertiary">No result yet</span>
						</div>
					{/if}
				</div>
			{:else}
				<!-- Simplified view for non-API modes -->
				<div class="space-y-2">
					{#if isLoading}
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 flex items-center gap-2 text-tertiary">
							<Loader2 class="w-3 h-3 animate-spin" />
							<span class="text-2xs">Executing...</span>
						</div>
					{:else if error}
						<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3">
							<pre class="text-2xs text-red-700 dark:text-red-300 whitespace-pre-wrap">{error}</pre>
						</div>
					{:else}
						<div class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 text-center">
							<span class="text-2xs text-green-600 dark:text-green-400">Tool executed successfully</span>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>