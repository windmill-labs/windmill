<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { twMerge } from 'tailwind-merge'
	import { Brain, ChevronDown, ChevronRight, Loader2 } from 'lucide-svelte'
	import type { DisplayMessage } from './shared'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import LinkRenderer from './LinkRenderer.svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		extractCandidatePaths,
		remarkWindmillPaths,
		workspaceItemRegistry
	} from './workspaceItems.svelte'

	interface Props {
		message: DisplayMessage
	}

	let { message }: Props = $props()

	const reasoning = $derived(message.role === 'assistant' ? message.reasoning : undefined)
	// Reasoning is still streaming when there's thinking text but no answer yet.
	const reasoningStreaming = $derived(!!reasoning && !message.content)
	// Expand while still thinking, collapse once the answer begins — unless toggled.
	let reasoningToggled = $state<boolean | undefined>(undefined)
	const reasoningExpanded = $derived(reasoningToggled ?? reasoningStreaming)

	const candidatePaths = $derived(extractCandidatePaths(message.content))
	const rendererPlugin = {
		renderer: {
			pre: CodeDisplay,
			a: LinkRenderer
		}
	}

	// Only populate the registry for messages that contain path-shaped tokens. The
	// registry still dedups concurrent calls across messages and workspaces.
	$effect(() => {
		const ws = $workspaceStore
		if (ws && candidatePaths.length > 0) workspaceItemRegistry.ensureLoaded(ws)
	})

	const plugins = $derived.by(() => {
		const ws = $workspaceStore ?? ''
		if (!ws || candidatePaths.length === 0) {
			return [gfmPlugin(), rendererPlugin]
		}

		if (!workspaceItemRegistry.isLoaded(ws)) {
			return [gfmPlugin(), rendererPlugin]
		}

		return [
			gfmPlugin(),
			{
				remarkPlugin: remarkWindmillPaths({
					resolve: (path) => workspaceItemRegistry.resolve(ws, path),
					workspace: ws || undefined
				}),
				renderer: {}
			},
			rendererPlugin
		]
	})
</script>

{#if reasoning}
	<div
		class="mb-2 bg-surface border border-border-light rounded-md overflow-hidden font-mono text-xs"
	>
		<button
			class={twMerge(
				'w-full p-2 bg-surface-secondary/30 hover:bg-surface-hover transition-colors flex items-center gap-2 text-left',
				reasoningExpanded ? 'border-b border-border-light' : ''
			)}
			onclick={() => (reasoningToggled = !reasoningExpanded)}
		>
			{#if reasoningExpanded}
				<ChevronDown class="w-3 h-3 text-secondary" />
			{:else}
				<ChevronRight class="w-3 h-3 text-secondary" />
			{/if}
			{#if reasoningStreaming}
				<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
			{:else}
				<Brain class="w-3.5 h-3.5 text-secondary" />
			{/if}
			<span class="text-primary font-medium text-2xs">Thinking</span>
		</button>

		{#if reasoningExpanded}
			<div class="p-2 bg-surface text-2xs text-secondary whitespace-pre-wrap break-words">
				{reasoning}
			</div>
		{/if}
	</div>
{/if}

{#if message.content}
	<div
		class="prose prose-sm dark:prose-invert w-full max-w-full leading-snug space-y-2 prose-ul:!pl-6
			prose-p:text-xs prose-li:text-xs prose-code:text-xs prose-pre:text-xs
			prose-code:break-words prose-a:break-words
			prose-headings:font-medium prose-headings:text-emphasis prose-headings:mt-3 prose-headings:mb-1
			prose-h1:text-sm prose-h2:text-xs prose-h3:text-xs prose-h4:text-xs prose-h5:text-xs prose-h6:text-xs
			prose-table:block prose-table:max-w-full prose-table:overflow-x-auto prose-table:text-xs"
	>
		<Markdown md={message.content} {plugins} />
	</div>
{/if}
