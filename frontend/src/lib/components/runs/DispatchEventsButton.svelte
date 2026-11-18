<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DispatchEventsTable from './DispatchEventsTable.svelte'
	import { useDispatchEvents } from './useDispatchEvents.svelte'
	import { GitFork } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	// Inline affordance for "this job triggered N other jobs". Renders nothing
	// when the job didn't dispatch anything so consumers can drop it unconditionally
	// next to other run-detail UI without worrying about an empty popover.
	type Props = { workspace: string; jobId: string; class?: string }
	let { workspace, jobId, class: klass = '' }: Props = $props()

	const events = useDispatchEvents(
		() => workspace,
		() => jobId
	)
	const list = $derived(events.list)
</script>

{#if list.length > 0}
	<Popover
		floatingConfig={{ strategy: 'fixed', placement: 'bottom-end', offset: { mainAxis: 8 } }}
		contentClasses="max-w-[640px] max-h-[420px] overflow-auto"
	>
		{#snippet trigger()}
			<button
				type="button"
				class={twMerge(
					'inline-flex items-center gap-1 px-2 py-0.5 rounded-sm text-3xs',
					'bg-surface-secondary hover:bg-surface-hover text-secondary border border-gray-200 dark:border-gray-700',
					klass
				)}
				title="View jobs this run dispatched"
			>
				<GitFork size={12} />
				<span>{list.length} dispatched</span>
			</button>
		{/snippet}
		{#snippet content()}
			<div class="overflow-hidden">
				<DispatchEventsTable events={list} {workspace} />
			</div>
		{/snippet}
	</Popover>
{/if}
