<script lang="ts">
	import { HistoryIcon } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import PopoverV2 from '$lib/components/meltComponents/Popover.svelte'
	import HistoricInputs from './HistoricInputs.svelte'
	import { workspaceStore } from '$lib/stores'
	import { JobService } from '$lib/gen'

	export let path: string
	export let selected: string | undefined = undefined
	export let selectInitial: boolean = false
	const dispatch = createEventDispatcher()

	async function loadInitial() {
		let jobs = await JobService.listJobs({
			workspace: $workspaceStore!,
			scriptPathExact: path,
			jobKinds: ['flow', 'flowpreview'].join(','),
			page: 1,
			perPage: 1
		})
		if (jobs.length > 0) {
			if (selectInitial) {
				dispatch('select', { jobId: jobs[0].id, initial: true })
			}
		} else {
			dispatch('nohistory')
		}
	}

	$: $workspaceStore && loadInitial()
</script>

<PopoverV2 closeButton={false}>
	<svelte:fragment slot="trigger">
		<HistoryIcon size={14} />
	</svelte:fragment>
	<svelte:fragment slot="content">
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
	</svelte:fragment>
</PopoverV2>
