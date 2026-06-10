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

	// Trimmed: rendered in a whitespace-pre-wrap block, so provider-sent leading/
	// trailing whitespace would otherwise show as a stray space before the content.
	const reasoning = $derived(
		message.role === 'assistant' ? message.reasoning?.trim() || undefined : undefined
	)
	// Some reasoning models (e.g. OpenAI) return no summary text — show a live indicator.
	const reasoningActive = $derived(message.role === 'assistant' && !!message.reasoningActive)
	// Show the block when there's thinking text, or while a textless model reasons pre-answer.
	const showThinking = $derived(!!reasoning || (reasoningActive && !message.content))
	// Spinner while reasoning hasn't produced the answer yet.
	const thinkingInProgress = $derived(!message.content && (reasoningActive || !!reasoning))
	// Expand while still thinking, collapse once the answer begins — unless toggled.
	let reasoningToggled = $state<boolean | undefined>(undefined)
	const reasoningExpanded = $derived(reasoningToggled ?? thinkingInProgress)

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

{#if showThinking}
	<div
		class="mb-2 bg-surface border border-border-light rounded-md overflow-hidden font-mono text-xs"
	>
		<button
			class={twMerge(
				'w-full p-2 bg-surface-secondary/30 hover:bg-surface-hover transition-colors flex items-center gap-2 text-left',
				reasoningExpanded && reasoning ? 'border-b border-border-light' : ''
			)}
			onclick={() => reasoning && (reasoningToggled = !reasoningExpanded)}
			disabled={!reasoning}
		>
			{#if reasoning}
				{#if reasoningExpanded}
					<ChevronDown class="w-3 h-3 text-secondary" />
				{:else}
					<ChevronRight class="w-3 h-3 text-secondary" />
				{/if}
			{/if}
			{#if thinkingInProgress}
				<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
			{:else}
				<Brain class="w-3.5 h-3.5 text-secondary" />
			{/if}
			<span class="text-primary font-medium text-2xs">Thinking</span>
		</button>

		{#if reasoningExpanded && reasoning}
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
