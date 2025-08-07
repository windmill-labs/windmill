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
	}
	
	let { 
		toolName, 
		description = '',
		parameters = {},
		result = undefined,
		isLoading = false,
		error = undefined,
		collapsed = false
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
	
	// Format JSON for display with syntax highlighting
	function formatJson(obj: any): string {
		try {
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
	
	// Simple JSON syntax highlighter
	function highlightJson(json: string): string {
		return json
			.replace(/(".*?":\s*)(".*?")/g, '$1<span class="json-string">$2</span>')
			.replace(/(".*?":\s*)(\d+)/g, '$1<span class="json-number">$2</span>')
			.replace(/(".*?":\s*)(true|false)/g, '$1<span class="json-boolean">$2</span>')
			.replace(/(".*?":\s*)(null)/g, '$1<span class="json-null">$2</span>')
			.replace(/"([^"]+)":/g, '<span class="json-key">"$1"</span>:')
	}
</script>

<div class="tool-execution-display">
	<!-- Collapsed/Expanded Header -->
	<div class="tool-header">
		<button 
			class="tool-header-button"
			onclick={() => isExpanded = !isExpanded}
		>
			<div class="flex items-center gap-2 flex-1">
				{#if isExpanded}
					<ChevronDown class="w-3 h-3 text-gray-500" />
				{:else}
					<ChevronRight class="w-3 h-3 text-gray-500" />
				{/if}
				
				{#if isLoading}
					<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-400" />
				{:else if error}
					<span class="text-red-500">✗</span>
				{:else if hasResult}
					<span class="text-green-500">✓</span>
				{:else}
					<span class="text-gray-500">○</span>
				{/if}
				
				<span class="tool-name">
					Called {displayName}
				</span>
				
				{#if isLoading}
					<span class="status-badge loading">Running</span>
				{:else if error}
					<span class="status-badge error">Failed</span>
				{:else if hasResult}
					<span class="status-badge success">Completed</span>
				{/if}
			</div>
		</button>
	</div>
	
	<!-- Expanded Content -->
	{#if isExpanded}
		<div class="tool-content">
			{#if description}
				<div class="description-section">
					{description}
				</div>
			{/if}
			
			<!-- Parameters Section -->
			{#if hasParameters}
				<div class="section">
					<div class="section-header">
						<span class="section-title">Parameters:</span>
						<button 
							class="copy-button"
							onclick={() => copyToClipboard(formatJson(parameters), 'params')}
							title="Copy parameters"
						>
							{#if copiedParams}
								<Check class="w-3 h-3" />
							{:else}
								<Copy class="w-3 h-3" />
							{/if}
						</button>
					</div>
					<div class="code-block">
						<pre><code>{@html highlightJson(formatJson(parameters))}</code></pre>
					</div>
				</div>
			{/if}
			
			<!-- Result Section -->
			<div class="section">
				<div class="section-header">
					<span class="section-title">Result:</span>
					{#if hasResult && !error}
						<button 
							class="copy-button"
							onclick={() => copyToClipboard(formatJson(result), 'result')}
							title="Copy result"
						>
							{#if copiedResult}
								<Check class="w-3 h-3" />
							{:else}
								<Copy class="w-3 h-3" />
							{/if}
						</button>
					{/if}
				</div>
				
				{#if isLoading}
					<div class="loading-state">
						<Loader2 class="w-3 h-3 animate-spin" />
						<span>Executing...</span>
					</div>
				{:else if error}
					<div class="code-block error-block">
						<pre><code>{error}</code></pre>
					</div>
				{:else if hasResult}
					<div class="code-block">
						<pre><code>{@html highlightJson(formatJson(result))}</code></pre>
					</div>
				{:else}
					<div class="empty-state">
						No result yet
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.tool-execution-display {
		background: #1a1a1a;
		border: 1px solid #333;
		border-radius: 6px;
		overflow: hidden;
		font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace;
		font-size: 13px;
	}
	
	.tool-header {
		background: #222;
		border-bottom: 1px solid #333;
	}
	
	.tool-header-button {
		width: 100%;
		padding: 10px 12px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: none;
		border: none;
		color: #e0e0e0;
		cursor: pointer;
		text-align: left;
		transition: background-color 0.15s;
	}
	
	.tool-header-button:hover {
		background: #2a2a2a;
	}
	
	.tool-name {
		font-weight: 500;
		color: #e0e0e0;
	}
	
	.status-badge {
		padding: 2px 8px;
		border-radius: 4px;
		font-size: 11px;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}
	
	.status-badge.loading {
		background: #2563eb20;
		color: #60a5fa;
		border: 1px solid #2563eb40;
	}
	
	.status-badge.success {
		background: #16a34a20;
		color: #4ade80;
		border: 1px solid #16a34a40;
	}
	
	.status-badge.error {
		background: #dc262620;
		color: #f87171;
		border: 1px solid #dc262640;
	}
	
	.tool-content {
		padding: 12px;
		background: #1a1a1a;
	}
	
	.description-section {
		padding: 8px 12px;
		background: #252525;
		border-radius: 4px;
		color: #999;
		font-size: 12px;
		margin-bottom: 12px;
	}
	
	.section {
		margin-bottom: 12px;
	}
	
	.section:last-child {
		margin-bottom: 0;
	}
	
	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 6px;
	}
	
	.section-title {
		color: #888;
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}
	
	.copy-button {
		padding: 4px;
		background: none;
		border: none;
		color: #666;
		cursor: pointer;
		border-radius: 3px;
		transition: all 0.15s;
	}
	
	.copy-button:hover {
		background: #333;
		color: #999;
	}
	
	.code-block {
		background: #0d0d0d;
		border: 1px solid #2a2a2a;
		border-radius: 4px;
		padding: 10px 12px;
		overflow-x: auto;
		max-height: 300px;
		overflow-y: auto;
	}
	
	.code-block pre {
		margin: 0;
		white-space: pre-wrap;
		word-wrap: break-word;
	}
	
	.code-block code {
		color: #e0e0e0;
		font-size: 12px;
		line-height: 1.5;
	}
	
	.error-block {
		background: #1a0d0d;
		border-color: #4a1f1f;
	}
	
	.error-block code {
		color: #f87171;
	}
	
	.loading-state {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px;
		background: #0d0d0d;
		border: 1px solid #2a2a2a;
		border-radius: 4px;
		color: #666;
		font-size: 12px;
	}
	
	.empty-state {
		padding: 12px;
		background: #0d0d0d;
		border: 1px solid #2a2a2a;
		border-radius: 4px;
		color: #555;
		font-size: 12px;
		text-align: center;
	}
	
	/* JSON Syntax Highlighting */
	:global(.json-key) {
		color: #9cdcfe;
	}
	
	:global(.json-string) {
		color: #ce9178;
	}
	
	:global(.json-number) {
		color: #b5cea8;
	}
	
	:global(.json-boolean) {
		color: #569cd6;
	}
	
	:global(.json-null) {
		color: #569cd6;
	}
	
	/* Scrollbar styling */
	.code-block::-webkit-scrollbar {
		width: 6px;
		height: 6px;
	}
	
	.code-block::-webkit-scrollbar-track {
		background: transparent;
	}
	
	.code-block::-webkit-scrollbar-thumb {
		background: #444;
		border-radius: 3px;
	}
	
	.code-block::-webkit-scrollbar-thumb:hover {
		background: #555;
	}
</style>