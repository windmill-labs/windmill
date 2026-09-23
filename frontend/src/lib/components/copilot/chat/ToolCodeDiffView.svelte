<script lang="ts">
	import type { ToolCodeDiff } from './shared'
	import {
		toolDiffLines,
		visibleToolDiffRows,
		type CharacterRange,
		type ToolDiffLine
	} from './toolCodeDiff'
	import { toolCodeDiffLanguage } from './toolCodeDiffLanguage'
	import { highlightedSourceLines } from './toolCodeDiffHighlight'
	import hljs from 'highlight.js/lib/core'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'
	import { Button } from '$lib/components/common'

	interface Props {
		diff: ToolCodeDiff
		streaming?: boolean
		diffLines?: ToolDiffLine[]
	}

	let { diff, streaming = false, diffLines }: Props = $props()

	let expandedSections = $state<Set<string>>(new Set())
	const diffRows = $derived(diffLines ?? toolDiffLines(diff, streaming))
	const visibleRows = $derived(visibleToolDiffRows(diffRows, expandedSections))
	const language = $derived(toolCodeDiffLanguage(diff.lang))
	const highlighted = $derived.by(() => {
		if (!hljs.getLanguage(language.name)) hljs.registerLanguage(language.name, language.register)

		let lastOldLine = 0
		let lastNewLine = 0
		for (const row of visibleRows) {
			if ('oldLine' in row && row.oldLine) lastOldLine = Math.max(lastOldLine, row.oldLine)
			if ('newLine' in row && row.newLine) lastNewLine = Math.max(lastNewLine, row.newLine)
		}
		return {
			original: highlightedSourceLines(diff.before, language.name, lastOldLine),
			modified: highlightedSourceLines(diff.after, language.name, lastNewLine)
		}
	})

	function highlightedLine(row: ToolDiffLine): string {
		const source = row.kind === 'removed' ? highlighted.original : highlighted.modified
		const lineNumber = row.kind === 'removed' ? row.oldLine : row.newLine
		return lineNumber === undefined ? '' : (source[lineNumber - 1] ?? '')
	}

	function expand(key: string): void {
		expandedSections = new Set(expandedSections).add(key)
	}

	function displayColumns(content: string): number {
		let columns = 0
		for (const character of content) {
			columns += character === '\t' ? 4 - (columns % 4) : 1
		}
		return columns
	}

	function rangeStyle(content: string, range: CharacterRange): string {
		const left = displayColumns(content.slice(0, range.start))
		const width = range.extendsToEnd
			? 'calc(100% - ' + left + 'ch)'
			: displayColumns(content.slice(range.start, range.start + range.length)) + 'ch'
		return 'left: ' + left + 'ch; width: ' + width
	}
</script>

<HighlightTheme />

<div
	class="tool-code-diff max-h-[400px] overflow-auto bg-surface-tertiary text-xs leading-[18px] text-primary"
>
	{#each visibleRows as row, index ('key' in row ? `${row.kind}:${row.key}` : `line:${row.oldLine}:${row.newLine}:${index}`)}
		{#if row.kind === 'omitted'}
			<div
				class="grid min-h-7 grid-cols-[0.875rem_2.9375rem_minmax(0,1fr)] items-center bg-surface-secondary py-1 text-hint"
			>
				<span></span>
				<span></span>
				<span>{row.count} more lines not shown</span>
			</div>
		{:else if row.kind === 'collapsed'}
			<Button
				unifiedSize="2xs"
				variant="subtle"
				btnClasses="grid min-h-7 w-full grid-cols-[0.875rem_2.9375rem_minmax(0,1fr)] items-center gap-0 rounded-none bg-surface-secondary px-0 py-1 text-left text-xs font-normal leading-[18px] text-hint hover:bg-surface-hover"
				onclick={() => expand(row.key)}
			>
				<span class="text-right">...</span>
				<span class="grid grid-cols-[1.125rem_0.875rem_0.9375rem]">
					<span></span><span class="text-right">...</span><span>...</span>
				</span>
				<span>{row.count} unchanged lines</span>
			</Button>
		{:else}
			<div class="diff-line-{row.kind} grid grid-cols-[0.875rem_2.9375rem_minmax(0,1fr)]">
				<span class="text-right text-hint">{row.oldLine ?? ''}</span>
				<span class="grid grid-cols-[1.125rem_0.875rem_0.9375rem] text-hint">
					<span></span><span class="text-right">{row.newLine ?? ''}</span><span
						>{row.kind === 'added' ? '+' : row.kind === 'removed' ? '-' : ''}</span
					>
				</span>
				<span
					class="relative min-w-0 whitespace-pre {row.kind === 'added'
						? 'bg-green-500/20'
						: row.kind === 'removed'
							? 'bg-red-500/20'
							: ''}"
				>
					{#each row.changedRanges ?? [] as range}
						<span
							class="pointer-events-none absolute top-0 z-0 h-[18px] {row.kind === 'added'
								? 'bg-green-500/25'
								: 'bg-red-500/20'}"
							style={rangeStyle(row.content, range)}
						></span>
					{/each}
					<span class="relative z-[1]">{@html highlightedLine(row) || '&nbsp;'}</span>
				</span>
			</div>
		{/if}
	{/each}
</div>

<style>
	.tool-code-diff {
		font-family: Menlo, Monaco, 'Courier New', monospace;
		/* Must match the tab stops `displayColumns` uses to place intraline highlights. */
		tab-size: 4;
	}

	.diff-line-added > span:nth-child(-n + 2),
	.diff-line-removed > span:nth-child(-n + 2) {
		@apply text-hint;
	}

	.tool-code-diff :global(.hljs-keyword),
	.tool-code-diff :global(.hljs-literal),
	.tool-code-diff :global(.hljs-built_in) {
		@apply text-blue-700;
	}

	.tool-code-diff :global(.hljs-string) {
		@apply text-red-700;
	}

	.tool-code-diff :global(.hljs-title.class_),
	.tool-code-diff :global(.hljs-type) {
		@apply text-cyan-800;
	}

	.tool-code-diff :global(.hljs-title.function_) {
		@apply text-primary;
	}

	.tool-code-diff :global(.hljs-attr),
	.tool-code-diff :global(.hljs-subst) {
		@apply text-primary;
	}

	.tool-code-diff :global(.hljs-number) {
		@apply text-green-700;
	}
</style>
