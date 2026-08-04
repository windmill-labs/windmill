<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Brain, Loader2 } from 'lucide-svelte'
	import type { DisplayMessage } from './shared'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import LinkRenderer from './LinkRenderer.svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		extractCandidatePaths,
		remarkWindmillPaths,
		workspaceItemRegistry
	} from './workspaceItems.svelte'
	import { markdownProse } from '$lib/components/markdownProse'

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
	<ChatCollapsibleCard
		label="Thinking"
		expanded={reasoningExpanded}
		onToggle={() => (reasoningToggled = !reasoningExpanded)}
		class="mb-2"
		contentClass="font-sans text-secondary {markdownProse.xs}"
	>
		{#snippet icon()}
			{#if reasoningStreaming}
				<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500 shrink-0" />
			{:else}
				<Brain class="w-3 h-3 text-secondary shrink-0" />
			{/if}
		{/snippet}
		<Markdown md={reasoning} plugins={[gfmPlugin()]} />
	</ChatCollapsibleCard>
{/if}

{#if message.content}
	<div class="w-full space-y-2 {markdownProse.sm}">
		<Markdown md={message.content} {plugins} />
	</div>
{/if}
