<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { CheckCircle2, Loader2, XCircle } from 'lucide-svelte'
	import type { RunnableRunState } from './activeRunnables.svelte'

	let { runState: rs, class: className }: { runState: RunnableRunState; class?: string } = $props()
</script>

<div
	class={twMerge(
		'shrink-0 flex items-center gap-0.5 px-1 py-0.5 rounded-sm border',
		rs.status === 'running'
			? 'bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 border-blue-300 dark:border-blue-700'
			: rs.status === 'success'
				? 'bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300 border-emerald-300 dark:border-emerald-700'
				: 'bg-red-100 dark:bg-red-900/40 text-red-700 dark:text-red-300 border-red-300 dark:border-red-700',
		className
	)}
	title={`${
		rs.status === 'running'
			? 'Running now'
			: rs.status === 'success'
				? 'Last run succeeded'
				: 'Last run failed'
	}${rs.runs > 0 ? ` — ran ${rs.runs}× this session` : ''}`}
>
	{#if rs.status === 'running'}
		<Loader2 size={10} class="animate-spin" />
	{:else if rs.status === 'success'}
		<CheckCircle2 size={10} />
	{:else}
		<XCircle size={10} />
	{/if}
	{#if rs.runs > 0}
		<span class="text-3xs leading-none tabular-nums">×{rs.runs}</span>
	{/if}
</div>
