<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Check, Circle, CircleDashed } from 'lucide-svelte'
	import type { SessionTasksStore } from './tasksState.svelte'

	// `loading` is the agent's turn state, not the task's: keying the pulse off
	// `in_progress` would leave a stalled or awaiting-input session animating forever.
	let { store, loading = false }: { store: SessionTasksStore; loading?: boolean } = $props()

	const tasks = $derived(store.tasks)
	const done = $derived(tasks.filter((t) => t.status === 'completed').length)
	const active = $derived(store.activeTasks)
	const allDone = $derived(tasks.length > 0 && done === tasks.length)

	const label = $derived(
		`Plan: ${done} of ${tasks.length} done` +
			(active.length ? `, currently ${active.map((t) => t.subject).join(', ')}` : '')
	)
</script>

{#if tasks.length > 0}
	<!-- bottom-end pins the peek's right edge to the trigger's so it grows leftward into
	     the chat. Opening rightward would lay it over the preview panel. -->
	<Popover
		openOnHover
		debounceDelay={50}
		placement="bottom-end"
		contentClasses="p-0"
		class="flex shrink-0 items-center gap-1.5 rounded px-1.5 py-0.5 hover:bg-surface-hover"
		triggerAttrs={{ 'aria-label': label }}
	>
		{#snippet trigger()}
			<!-- Fixed width with the segments flexing inside it: a task added mid-run must
			     not reflow the elastic session name beside it. Past ~8 the gaps would eat
			     more width than the segments, so they tighten. -->
			<span
				class="flex w-14 shrink-0 {tasks.length > 8 ? 'gap-px' : 'gap-[1.5px]'}"
				aria-hidden="true"
			>
				{#each tasks as task (task.seq)}
					<span
						class="h-1.5 flex-1 rounded-[1px] {task.status === 'completed'
							? 'bg-green-500'
							: task.status === 'in_progress'
								? `bg-indigo-500 ${loading ? 'motion-safe:animate-pulse' : ''}`
								: 'bg-gray-200 dark:bg-gray-600'}"
					></span>
				{/each}
			</span>
			<span
				class="text-2xs font-medium tabular-nums {allDone
					? 'text-green-600 dark:text-green-400'
					: 'text-secondary'}"
			>
				{done}/{tasks.length}
			</span>
		{/snippet}
		{#snippet content()}
			<div class="w-72 max-w-[80vw] p-1">
				<div
					class="flex items-center justify-between px-1.5 pb-1 pt-0.5 text-2xs uppercase tracking-wide text-hint"
				>
					<span>Plan</span>
					<span class="tabular-nums">{done} of {tasks.length}</span>
				</div>
				<div class="flex max-h-64 flex-col overflow-y-auto">
					{#each tasks as task (task.seq)}
						<div
							class="flex items-center gap-1.5 rounded px-1.5 py-1 text-xs {task.status ===
							'in_progress'
								? 'bg-indigo-50 font-medium dark:bg-indigo-500/15'
								: ''}"
						>
							{#if task.status === 'completed'}
								<Check size={12} class="shrink-0 text-green-500" />
							{:else if task.status === 'in_progress'}
								<!-- Solid against the pending dashed ring: at 12px the two must differ in
								     silhouette, not just hue. Pulses with the bar's running segment. -->
								<Circle
									size={12}
									class="shrink-0 fill-indigo-500 text-indigo-500 {loading
										? 'motion-safe:animate-pulse'
										: ''}"
								/>
							{:else}
								<CircleDashed size={12} class="shrink-0 text-hint" />
							{/if}
							<span class="w-4 shrink-0 text-right text-2xs tabular-nums text-hint">{task.seq}</span
							>
							<span
								class="min-w-0 flex-1 truncate {task.status === 'completed'
									? 'text-hint line-through'
									: 'text-primary'}"
								title={task.description}
							>
								<!-- The active row reads in the present continuous, which is what
								     activeForm is for; the others state the task itself. -->
								{task.status === 'in_progress' ? (task.activeForm ?? task.subject) : task.subject}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/snippet}
	</Popover>
{/if}
