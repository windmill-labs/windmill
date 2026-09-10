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
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { ExternalLink } from 'lucide-svelte'
	import CopyButton from '$lib/components/common/button/CopyButton.svelte'
	import { base } from '$lib/base'
	import { displayDate } from '$lib/utils'

	interface Props {
		message: DisplayMessage
		// Workspace the message's paths are resolved against: the one the chat
		// operates on, which is not always the one being navigated.
		workspace: string | undefined
	}

	let { message, workspace }: Props = $props()

	// The run this answer came out of. Only a flow chat has one — a copilot turn runs in
	// the browser — so the footer is absent rather than empty elsewhere.
	const jobId = $derived(message.role === 'assistant' ? message.jobId : undefined)
	const createdAt = $derived(message.role === 'assistant' ? message.createdAt : undefined)
	const runHref = $derived(jobId ? `${base}/run/${jobId}?workspace=${workspace}` : undefined)
	// Today's answers show the time alone; the day earns its place only on a conversation
	// read back later. Resolved at render, so a chat left open across midnight keeps
	// yesterday's format until it is reopened.
	const timestamp = $derived.by(() => {
		if (!createdAt) return undefined
		const at = new Date(createdAt)
		const today = new Date().toDateString() === at.toDateString()
		return displayDate(at, false, !today)
	})

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

	const stepName = $derived(message.role === 'assistant' ? message.stepName : undefined)

	// A flow step can return a file rather than text; the raw JSON would be
	// unreadable, so hand it to the result viewer instead of the markdown renderer.
	const s3Object = $derived.by(() => {
		if (!message.content.startsWith('{')) return undefined
		try {
			const parsed = JSON.parse(message.content)
			return parsed?.type === 'windmill_s3_object' && parsed?.s3 ? parsed : undefined
		} catch {
			return undefined
		}
	})

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

{#if stepName}
	<div class="text-2xs text-tertiary font-medium mb-1">{stepName}</div>
{/if}

{#if s3Object}
	<DisplayResult result={s3Object} workspaceId={workspace} noControls={true} />
{:else if message.content}
	<div class="w-full space-y-2 {markdownProse.sm}">
		<Markdown md={message.content} {plugins} />
	</div>
{/if}

{#if message.content}
	<!-- Present but invisible until the answer is hovered: kept in flow so revealing it
	     does not nudge the message below, and with no margin of its own so it sits in the
	     gap the transcript already leaves between messages. A row carrying only thinking
	     has no answer to copy or date, and the run behind it is the one the next row
	     already links. -->
	<div
		class="flex items-center gap-2 text-2xs text-tertiary opacity-0 transition-opacity duration-150 group-hover/answer:opacity-100 focus-within:opacity-100"
	>
		{#if message.content}
			<CopyButton value={message.content} title="Copy answer" class="-ml-1" />
		{/if}
		{#if timestamp}
			<span>{timestamp}</span>
		{/if}
		{#if runHref}
			<a
				href={runHref}
				target="_blank"
				rel="noopener noreferrer"
				class="inline-flex items-center gap-1 hover:text-primary hover:underline"
				title="Open this run"
			>
				<span>job <span class="font-mono">{jobId?.slice(0, 8)}</span></span>
				<ExternalLink size={11} class="shrink-0" />
			</a>
		{/if}
	</div>
{/if}
