<script lang="ts">
	import { Loader2, Copy, Check } from 'lucide-svelte'
	import { TOOL_PRETTIFY_MAP } from './shared'

	interface Props {
		title: string
		content?: any
		error?: string
		loading?: boolean
		showCopy?: boolean
		showWhileLoading?: boolean
		streaming?: boolean
		toolName?: string
		showFade?: boolean
	}

	let {
		title,
		content,
		error,
		loading,
		showCopy = true,
		showWhileLoading = true,
		streaming = false,
		toolName,
		showFade = false
	}: Props = $props()
	let copied = $state(false)

	const hasContent = $derived(content !== undefined && content !== null)

	// Look up prettify function from the map using toolName
	const prettifyFn = $derived(toolName ? TOOL_PRETTIFY_MAP[toolName] : undefined)

	function formatJson(obj: any): string {
		try {
			// Apply prettify function if available for this tool
			if (prettifyFn) {
				if (typeof obj === 'object') {
					return prettifyFn(JSON.stringify(obj, null, 2))
				} else {
					return prettifyFn(obj)
				}
			}

			// Original formatting logic as fallback
			if (typeof obj === 'string') {
				try {
					const parsed = JSON.parse(obj)
					return JSON.stringify(parsed, null, 2)
				} catch {
					return obj
				}
			}
			for (const key in obj) {
				try {
					const parsed = JSON.parse(obj[key])
					obj[key] = parsed
				} catch {}
			}
			return JSON.stringify(obj, null, 2)
		} catch {
			return String(obj)
		}
	}

	async function copyToClipboard() {
		if (!hasContent) return
		try {
			await navigator.clipboard.writeText(formatJson(content))
			copied = true
			setTimeout(() => (copied = false), 1500)
		} catch (err) {
			console.error('Failed to copy:', err)
		}
	}
</script>

{#if showWhileLoading || (!loading && hasContent) || streaming}
	<div class="space-y-2">
		<div class="flex items-center justify-between">
			<span class="text-2xs">
				{title}:
			</span>
			{#if showCopy && hasContent && !streaming}
				<button
					class="p-1 rounded hover:bg-surface-secondary text-primary hover:text-secondary transition-colors"
					onclick={copyToClipboard}
					title="Copy {title.toLowerCase()}"
				>
					{#if copied}
						<Check class="w-3 h-3 text-green-500" />
					{:else}
						<Copy class="w-3 h-3" />
					{/if}
				</button>
			{/if}
		</div>

		{#if loading && !streaming && !hasContent}
			<div
				class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 flex items-center gap-2 text-primary"
			>
				<Loader2 class="w-3 h-3 animate-spin" />
				<span class="text-2xs">Executing...</span>
			</div>
		{:else if error}
			<div
				class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3 overflow-x-auto max-h-28 overflow-y-auto"
			>
				<pre class="text-2xs text-red-700 dark:text-red-300 whitespace-pre-wrap">{error}</pre>
			</div>
		{:else if hasContent}
			<div
				class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded overflow-hidden relative"
			>
				<div class="p-3 overflow-x-auto max-h-28 overflow-y-auto">
					<pre class="text-2xs text-primary whitespace-pre-wrap"
						>{formatJson($state.snapshot(content))}</pre
					>
				</div>
				{#if showFade}
					<div
						class="absolute bottom-0 left-0 right-0 h-16 pointer-events-none bg-gradient-to-t from-surface-secondary via-surface-secondary/70 via-surface-secondary/40 to-transparent"
					></div>
				{/if}
			</div>
		{:else}
			<div
				class="bg-surface-secondary border border-gray-200 dark:border-gray-700 rounded p-3 text-center"
			>
				<span class="text-2xs text-primary">No {title.toLowerCase()} yet</span>
			</div>
		{/if}
	</div>
{/if}
