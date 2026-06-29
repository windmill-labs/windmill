<script lang="ts">
	import { Button } from '$lib/components/common'
	import {
		CheckCircle2,
		ChevronDown,
		ChevronUp,
		Clock,
		Code2,
		GitBranch,
		Loader2,
		XCircle
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { isActiveEvent, type PipelineEvent } from './activeRunnables.svelte'

	let {
		events
	}: {
		events: PipelineEvent[]
	} = $props()

	// Panel visibility only. The observe poll that feeds `events` (and the node
	// badges) runs whenever the graph is open, independent of this panel, so
	// the log is already populated when expanded and badges show while collapsed.
	let open = $state(false)

	// Excludes future-scheduled queued jobs (a schedule's next planned run
	// is not activity) — see isActiveEvent.
	let runningCount = $derived(events.filter((e) => isActiveEvent(e)).length)

	function ago(iso: string): string {
		const s = Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 1000))
		if (s < 60) return `${s}s ago`
		if (s < 3600) return `${Math.floor(s / 60)}m ago`
		if (s < 86400) return `${Math.floor(s / 3600)}h ago`
		return `${Math.floor(s / 86400)}d ago`
	}
</script>

<div
	class="absolute bottom-0 left-0 right-0 z-20 border-t border-gray-200 dark:border-gray-700 bg-surface/95 backdrop-blur-sm"
>
	<Button
		variant="subtle"
		unifiedSize="sm"
		onclick={() => (open = !open)}
		btnClasses="w-full !justify-between !rounded-none !px-3 !py-1.5"
	>
		<span class="flex items-center gap-2 text-xs font-medium text-secondary">
			<Clock size={13} />
			Activity
			{#if runningCount > 0}
				<span
					class="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 text-2xs"
				>
					<Loader2 size={9} class="animate-spin" />
					{runningCount} running
				</span>
			{:else if events.length > 0}
				<span class="text-2xs text-tertiary">{events.length}</span>
			{/if}
		</span>
		{#if open}
			<ChevronDown size={14} />
		{:else}
			<ChevronUp size={14} />
		{/if}
	</Button>

	{#if open}
		<!-- Cap the list and scroll past it: never taller than 18rem, and
		     never more than ~40% of the viewport so it can't swallow the
		     graph on a short screen. The header (Button) stays outside this
		     scroll container so it's always visible. -->
		<div class="max-h-[min(18rem,40vh)] overflow-y-auto px-1 pb-1">
			{#if events.length === 0}
				<div class="px-3 py-6 text-center text-xs text-tertiary">
					No activity yet — runs on this folder will appear here live.
				</div>
			{:else}
				{#each events as e (e.id)}
					<div class="flex items-center gap-2 px-2 py-1 rounded-sm hover:bg-surface-hover text-xs">
						<span class="shrink-0">
							{#if e.status === 'running'}
								<Loader2 size={12} class="animate-spin text-blue-600 dark:text-blue-400" />
							{:else if e.status === 'queued'}
								<Clock size={12} class="text-gray-500" />
							{:else if e.status === 'success'}
								<CheckCircle2 size={12} class="text-emerald-600 dark:text-emerald-400" />
							{:else}
								<XCircle size={12} class="text-red-600 dark:text-red-400" />
							{/if}
						</span>
						{#if e.kind === 'flow'}
							<GitBranch size={12} class="shrink-0 text-tertiary" />
						{:else}
							<Code2 size={12} class="shrink-0 text-tertiary" />
						{/if}
						<span class="flex-1 min-w-0 truncate font-mono text-2xs" title={e.path}>
							{e.path}
						</span>
						<span
							class={twMerge(
								'shrink-0 px-1 py-0.5 rounded-sm text-3xs leading-none',
								e.source === 'schedule'
									? 'bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300'
									: 'bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400'
							)}
						>
							{e.source}
						</span>
						<span class="shrink-0 text-3xs text-tertiary tabular-nums w-14 text-right">
							{ago(e.at)}
						</span>
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>
