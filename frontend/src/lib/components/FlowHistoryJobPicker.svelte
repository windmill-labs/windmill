<script lang="ts">
	import { HistoryIcon } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import PopoverV2 from '$lib/components/meltComponents/Popover.svelte'
	import HistoricInputs from './HistoricInputs.svelte'
	import { workspaceStore } from '$lib/stores'
	import { JobService } from '$lib/gen'
	import Button from '$lib/components/common/button/Button.svelte'

	export let path: string
	export let selected: string | undefined = undefined
	const dispatch = createEventDispatcher()
	export let floatingConfig: any | undefined = undefined
	export let height: number | undefined = undefined
	export let contentClasses: string | undefined = undefined


	async function loadInitial() {
		let jobs = await JobService.listJobs({
			workspace: $workspaceStore!,
			scriptPathExact: path,
			jobKinds: ['flow', 'flowpreview'].join(','),
			page: 1,
			perPage: 1
		})
		if (jobs.length > 0) {
			dispatch('select', { jobId: jobs[0].id, initial: true })
		} else {
			dispatch('nohistory')
		}
	}

	$: $workspaceStore && loadInitial()
</script>

<PopoverV2 closeButton={false} {floatingConfig} usePointerDownOutside {contentClasses}>
	<svelte:fragment slot="trigger">
		<Button
			color="light"
			size="xs2"
			variant="contained"
			btnClasses="bg-transparent"
			iconOnly
			nonCaptureEvent
			startIcon={{ icon: HistoryIcon }}
		/>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div
			class="p-2 {height ? '' : 'h-[400px]'} overflow-hidden w-80 shadow-sm"
			style={`height: ${height}px`}
		>
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
