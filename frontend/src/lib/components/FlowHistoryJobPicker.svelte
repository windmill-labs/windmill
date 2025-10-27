<script lang="ts">
	import { HistoryIcon } from 'lucide-svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import PopoverV2 from '$lib/components/meltComponents/Popover.svelte'
	import HistoricInputs from './HistoricInputs.svelte'
	import { workspaceStore } from '$lib/stores'
	import { JobService } from '$lib/gen'

	interface Props {
		path: string
		selected?: string | undefined
		selectInitial?: boolean
		loading?: boolean
	}

	let {
		path,
		selected = undefined,
		selectInitial = false,
		loading = $bindable(false)
	}: Props = $props()
	const dispatch = createEventDispatcher()

	async function loadInitial() {
		loading = true
		let jobs = await JobService.listJobs({
			workspace: $workspaceStore!,
			scriptPathExact: path,
			jobKinds: ['flow', 'flowpreview'].join(','),
			perPage: 1
		})
		if (jobs.length > 0) {
			if (selectInitial) {
				dispatch('select', { jobId: jobs[0].id, initial: true })
			}
		} else {
			dispatch('nohistory')
		}
		loading = false
	}

	$effect(() => {
		$workspaceStore && untrack(() => loadInitial())
	})
</script>

<PopoverV2 closeButton={false}>
	{#snippet trigger()}
		<HistoryIcon size={14} />
	{/snippet}
	{#snippet content()}
		<div class="p-2 h-[400px] overflow-hidden w-80 border shadow-sm">
			<HistoricInputs
				on:select={(e) => {
					if (e.detail) {
						dispatch('select', { jobId: e.detail?.jobId, initial: false })
					} else {
						dispatch('unselect')
					}
				}}
				{selected}
				runnableId={path}
				runnableType={'FlowPath'}
			/>
		</div>
	{/snippet}
</PopoverV2>
