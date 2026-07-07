<script lang="ts">
	import {
		AlertCircle,
		AlertTriangle,
		Building,
		GitFork,
		GitPullRequestClosed
	} from 'lucide-svelte'
	import type { SessionChatStatus } from './sessionRuntime.svelte'

	// The fork icon is deliberately sync-state-agnostic (no ahead/behind/
	// diverged variants — that detail lives in the fork bar). Its only special
	// state is detached: the session's fork workspace no longer exists.
	let {
		status,
		isFork,
		forkDetached = false
	}: { status: SessionChatStatus; isFork: boolean; forkDetached?: boolean } = $props()

	const statusTooltip: Record<SessionChatStatus, string> = {
		idle: 'No chat activity',
		streaming: 'Generating response…',
		'awaiting-user': 'Waiting for your reply',
		'needs-confirmation': 'Needs your confirmation',
		draft: 'Unsent draft',
		error: 'Last message had an error'
	}

	// Live chat signals take precedence over the persistent kind/fork
	// indicator: streaming, needs-confirmation, and error are time-critical
	// and warrant briefly hijacking the icon slot.
	const liveOverride = $derived(
		status === 'streaming' || status === 'needs-confirmation' || status === 'error'
	)

	const persistentTitle = $derived(
		isFork
			? forkDetached
				? 'Fork — no longer available'
				: 'Fork session'
			: 'Root workspace session'
	)

	const title = $derived(liveOverride ? statusTooltip[status] : persistentTitle)
</script>

<span class="inline-flex items-center justify-center w-4 h-3 shrink-0" {title}>
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
	{:else if isFork}
		{#if forkDetached}
			<GitPullRequestClosed class="w-3 h-3 text-red-500" />
		{:else}
			<GitFork class="w-3 h-3 text-tertiary" />
		{/if}
	{:else}
		<Building class="w-3 h-3 text-tertiary" />
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
