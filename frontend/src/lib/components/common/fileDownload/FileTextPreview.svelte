<script lang="ts" module>
	export function isMarkdownFile(fileKey: string | undefined): boolean {
		// Not `.markdown`: the file preview route reads it as an unknown type.
		return /\.md$/i.test(fileKey ?? '')
	}

	// The read stops at a byte count, so a truncated file usually ends mid-line.
	function dropPartialLastLine(text: string) {
		const end = text.lastIndexOf('\n')
		return end > 0 ? text.slice(0, end) : text
	}
</script>

<script lang="ts">
	import { Code, Eye, Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Highlight } from 'svelte-highlight'
	import { json as jsonLanguage } from 'svelte-highlight/languages'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	interface Props {
		/** The start of a file, as read by a file preview request. */
		content: string
		/** Bytes the request read at most: content this long is the start of a longer file. */
		readLimit: number
		/** Rendered as the session renders a markdown artifact, with a switch to its source. */
		markdown?: boolean
		/** Highlighted as JSON, and indented when the whole file was read. */
		json?: boolean
		/** Fill the parent's height, whose padding then sets the margins, instead of capping the
		 * text at 384px with margins of its own. Drawn flat on `surface-input` either way. */
		fill?: boolean
		/** Markdown shown as its source, owned by the caller, which then renders its own switch:
		 * the preview hides its own. */
		markdownSource?: boolean
	}

	let {
		content,
		readLimit,
		markdown = false,
		json = false,
		fill = false,
		markdownSource = undefined
	}: Props = $props()

	let localShowSource = $state(false)
	let showSource = $derived(markdownSource ?? localShowSource)
	let truncated = $derived(new TextEncoder().encode(content).length >= readLimit)
	let shown = $derived(truncated ? dropPartialLastLine(content) : content)
	let source = $derived(markdown && showSource)
	let jsonCode = $derived.by(() => {
		if (!json || truncated) return shown
		try {
			return JSON.stringify(JSON.parse(content), null, 2)
		} catch {
			return content
		}
	})
	// The source editor fills its box, so the box needs a height rather than a cap.
	let boxSize = $derived(fill ? 'flex-1 min-h-0' : source ? 'h-96' : 'max-h-96')
</script>

<div class={twMerge('flex flex-col min-w-0 bg-surface-input', fill ? 'h-full gap-2' : '')}>
	{#if markdown}
		{#if markdownSource === undefined}
			<!-- The session's own switch for a markdown artifact. -->
			<ToggleButtonGroup
				noWFull
				class="self-end"
				selected={showSource ? 'source' : 'preview'}
				onSelected={(v) => (localShowSource = v === 'source')}
			>
				{#snippet children({ item })}
					<ToggleButton {item} value="preview" icon={Eye} iconOnly tooltip="Preview" size="sm" />
					<ToggleButton
						{item}
						value="source"
						icon={Code}
						iconOnly
						tooltip="View source"
						size="sm"
					/>
				{/snippet}
			</ToggleButtonGroup>
		{/if}
		<div class={twMerge('overflow-auto', source ? '' : fill ? 'px-5' : 'px-8', boxSize)}>
			<!-- Lazy: the source view is a Monaco editor, and the logged-in layout mounts the S3
			     file browser that renders this. No `pre`: the default code block keeps mermaid off,
			     since a file or a job result is untrusted markdown. -->
			{#await import('$lib/components/copilot/chat/artifacts/ArtifactBody.svelte')}
				<div class="flex items-center gap-1 p-3 text-2xs text-secondary">
					<Loader2 size={12} class="animate-spin" /> Loading preview
				</div>
			{:then ArtifactBody}
				<ArtifactBody.default
					content={shown}
					{source}
					fadeFrom="from-surface-input"
					docPadding={fill ? 'pb-8 pt-2' : 'pb-8 pt-6'}
				/>
			{/await}
		</div>
	{:else}
		<div class={twMerge('overflow-auto', fill ? '' : 'px-3 py-2', boxSize)}>
			{#if json}
				<Highlight language={jsonLanguage} code={jsonCode} class="text-xs" />
			{:else}
				<pre class="text-xs font-mono text-primary whitespace-pre">{shown}</pre>
			{/if}
		</div>
	{/if}
	{#if truncated}
		<div class={twMerge('text-2xs text-hint', fill ? '' : 'px-3 pb-2')}>
			Showing the first {Math.round(readLimit / 1024)} KB. Download the file to see all of it.
		</div>
	{/if}
</div>
