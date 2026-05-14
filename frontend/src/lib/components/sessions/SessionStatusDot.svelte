<script lang="ts">
	import { AlertTriangle, AlertCircle, GitFork } from 'lucide-svelte'
	import type { SessionChatStatus } from './sessionRuntime.svelte'

	let { status, isFork = false }: { status: SessionChatStatus; isFork?: boolean } = $props()

	const tooltip: Record<SessionChatStatus, string> = {
		idle: 'No chat activity',
		streaming: 'Generating response…',
		'awaiting-user': 'Waiting for your reply',
		'needs-confirmation': 'Needs your confirmation',
		draft: 'Unsent draft',
		error: 'Last message had an error'
	}

	// Live statuses always trump the fork glyph — users need to see what
	// the session is actually doing right now. Otherwise, fork sessions
	// show GitFork as their persistent marker.
	const showForkIcon = $derived(
		isFork && status !== 'streaming' && status !== 'needs-confirmation' && status !== 'error'
	)
</script>

<span
	class="inline-flex items-center justify-center w-4 h-3 shrink-0"
	title={isFork ? `Fork — ${tooltip[status]}` : tooltip[status]}
>
	{#if status === 'streaming'}
		<span class="inline-flex items-end gap-0.5">
			<span class="w-1 h-1 rounded-full bg-blue-500 typing-dot"></span>
			<span class="w-1 h-1 rounded-full bg-blue-500 typing-dot dot-2"></span>
			<span class="w-1 h-1 rounded-full bg-blue-500 typing-dot dot-3"></span>
		</span>
	{:else if status === 'needs-confirmation'}
		<AlertCircle class="w-3 h-3 text-amber-500" />
	{:else if status === 'error'}
		<AlertTriangle class="w-3 h-3 text-red-500" />
	{:else if showForkIcon}
		<GitFork class="w-3 h-3 text-tertiary" />
	{:else if status === 'awaiting-user'}
		<span class="w-1.5 h-1.5 rounded-full bg-blue-500"></span>
	{:else if status === 'draft'}
		<span class="w-1.5 h-1.5 rounded-full bg-gray-400 opacity-60"></span>
	{:else}
		<span class="w-1.5 h-1.5 rounded-full border border-gray-400 opacity-50"></span>
	{/if}
</span>

<style>
	.typing-dot {
		animation: typing 1.2s ease-in-out infinite;
	}
	.dot-2 {
		animation-delay: 0.15s;
	}
	.dot-3 {
		animation-delay: 0.3s;
	}
	@keyframes typing {
		0%,
		60%,
		100% {
			opacity: 0.3;
			transform: translateY(0);
		}
		30% {
			opacity: 1;
			transform: translateY(-1px);
		}
	}
</style>
