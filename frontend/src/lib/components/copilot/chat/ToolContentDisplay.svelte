<script lang="ts">
	import { Loader2, Copy, Check } from 'lucide-svelte'
	import { TOOL_PRETTIFY_MAP } from './shared'
	import { scrollFades } from './scrollFades.svelte'

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
		/** Open on the end of the content instead of its start, and stay there as it grows.
		 * For logs, whose last lines are the ones being looked for. */
		tail?: boolean
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
		showFade = false,
		tail = false
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

	// Only draw the bottom fade when the content actually overflows and the user hasn't
	// scrolled to the bottom. `showFade` is the parent's intent; `fades.bottom` is the live
	// measurement on the inner scroll container, which catches streaming JSON growing past
	// max-h-28 as well as the first paint.
	const fades = scrollFades()
	const { container: fadeContainer, content: fadeContent, measure: measureFades } = fades

	let scroller = $state<HTMLDivElement | undefined>()
	$effect(() => {
		void content
		if (tail && scroller) scroller.scrollTop = scroller.scrollHeight
	})
</script>

{#if showWhileLoading || (!loading && hasContent) || streaming}
	<div class="space-y-2 group">
		<div class="flex items-center justify-between">
			<span class="text-2xs text-hint">
				{title}:
			</span>
			{#if showCopy && hasContent && !streaming}
				<button
					class="p-1 rounded hover:bg-surface-secondary text-primary hover:text-secondary transition-opacity duration-200 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
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
			<div class="flex items-center gap-2 text-primary">
				<Loader2 class="w-3 h-3 animate-spin" />
				<span class="text-2xs">Executing...</span>
			</div>
		{:else if error}
			<div class="overflow-x-auto max-h-28 overflow-y-auto">
				<pre class="text-2xs text-red-700 dark:text-red-300 whitespace-pre-wrap">{error}</pre>
			</div>
		{:else if hasContent}
			<div class="relative">
				<div
					bind:this={scroller}
					use:fadeContainer
					onscroll={measureFades}
					class="overflow-x-auto max-h-28 overflow-y-auto"
				>
					<pre use:fadeContent class="text-2xs text-primary whitespace-pre-wrap"
						>{formatJson($state.snapshot(content))}</pre
					>
				</div>
				{#if showFade && fades.bottom}
					<div
						class="absolute bottom-0 left-0 right-0 h-[min(2.5rem,35%)] pointer-events-none bg-gradient-to-t from-surface via-surface/70 to-transparent"
					></div>
				{/if}
			</div>
		{:else}
			<span class="text-2xs text-tertiary">No {title.toLowerCase()} yet</span>
		{/if}
	</div>
{/if}
