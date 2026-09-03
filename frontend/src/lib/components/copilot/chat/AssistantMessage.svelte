<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import type { DisplayMessage } from './shared'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import { thinkingPreferences } from './thinkingPreferences.svelte'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import LinkRenderer from './LinkRenderer.svelte'
	import {
		extractCandidatePaths,
		remarkWindmillPaths,
		workspaceItemRegistry
	} from './workspaceItems.svelte'
	import { markdownProse } from '$lib/components/markdownProse'

	interface Props {
		message: DisplayMessage
		// Workspace the message's paths are resolved against: the one the chat
		// operates on, which is not always the one being navigated.
		workspace: string | undefined
	}

	let { message, workspace }: Props = $props()

	const reasoning = $derived(
		message.role === 'assistant' ? message.reasoning?.trim() || undefined : undefined
	)
	// Set the moment thinking ends, which is mid-turn on the live message — the
	// answer streams on afterwards.
	const reasoningDurationMs = $derived(
		message.role === 'assistant' ? message.reasoningDurationMs : undefined
	)
	// Shimmer while the reasoning text streams before the answer. Only the live
	// synthetic message carries `streaming` — a finalized reasoning-only message
	// (thinking that led straight to a tool call) must not look in-progress.
	const reasoningStreaming = $derived(
		!!reasoning &&
			message.role === 'assistant' &&
			!!message.streaming &&
			!message.content &&
			reasoningDurationMs === undefined
	)
	// Undefined until this block is toggled by hand, so flipping the preference
	// reaches every block the reader hasn't already made a decision about.
	let reasoningToggled = $state<boolean | undefined>(undefined)
	const reasoningExpanded = $derived(reasoningToggled ?? thinkingPreferences.expandByDefault)
	const reasoningLabel = $derived(
		reasoningDurationMs !== undefined
			? `Thought for ${formatThinkingDuration(reasoningDurationMs)}`
			: reasoningStreaming
				? 'Thinking...'
				: 'Thinking'
	)

	function formatThinkingDuration(ms: number): string {
		const seconds = Math.max(1, Math.round(ms / 1000))
		if (seconds < 60) return `${seconds}s`
		const minutes = Math.floor(seconds / 60)
		const rest = seconds % 60
		return rest === 0 ? `${minutes}m` : `${minutes}m ${rest}s`
	}

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
		if (workspace && candidatePaths.length > 0) workspaceItemRegistry.ensureLoaded(workspace)
	})

	const plugins = $derived.by(() => {
		const ws = workspace ?? ''
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
		label={reasoningLabel}
		expanded={reasoningExpanded}
		onToggle={() => (reasoningToggled = !reasoningExpanded)}
		shimmer={reasoningStreaming}
		class="mb-2"
		labelClass="truncate"
		contentClass="font-main text-secondary {markdownProse.xs}"
	>
		<Markdown md={reasoning} plugins={[gfmPlugin()]} />
	</ChatCollapsibleCard>
{/if}

{#if message.content}
	<div class="w-full space-y-2 {markdownProse.sm}">
		<Markdown md={message.content} {plugins} />
	</div>
{/if}
