<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { twMerge } from 'tailwind-merge'
	import { Brain, ChevronDown, ChevronRight, Loader2 } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
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

	const reasoning = $derived(
		message.role === 'assistant' ? message.reasoning?.trim() || undefined : undefined
	)
	// Spinner while the reasoning text streams before the answer. Only the live
	// synthetic message carries `streaming` — a finalized reasoning-only message
	// (thinking that led straight to a tool call) must not look in-progress.
	const reasoningStreaming = $derived(
		!!reasoning && message.role === 'assistant' && !!message.streaming && !message.content
	)
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
	<div class="mb-2 bg-surface border border-border-light rounded-md overflow-hidden text-xs">
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
			<div
				transition:slide={{ duration: 150 }}
				class="p-2 bg-surface text-secondary break-words prose prose-sm dark:prose-invert max-w-full leading-snug
					prose-p:text-2xs prose-li:text-2xs prose-code:text-2xs prose-pre:text-2xs prose-ul:!pl-5
					prose-headings:font-medium prose-headings:text-secondary prose-headings:mt-2 prose-headings:mb-1
					prose-h1:text-2xs prose-h2:text-2xs prose-h3:text-2xs prose-h4:text-2xs prose-h5:text-2xs prose-h6:text-2xs
					prose-strong:text-secondary"
			>
				<Markdown md={reasoning} plugins={[gfmPlugin()]} />
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
