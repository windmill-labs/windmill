<script lang="ts">
	import { RefreshCw } from 'lucide-svelte'
	import { Button } from '../common'
	import { createEventDispatcher } from 'svelte'

	export let minTs: string | undefined
	export let maxTs: string | undefined
	export let loading: boolean = false
	export let selectedManualDate = 0

	const manualDates = [
		{
			label: 'Last 1000 runs',
			setMinMax: () => {
				minTs = undefined
				maxTs = undefined
			}
		},
		{
			label: 'Within 30 seconds',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last minute',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last 5 minutes',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 5 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last 30 minutes',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last 24 hours',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last 7 days',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 7 * 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last month',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		}
	]

	const dispatch = createEventDispatcher()
</script>

<Button
	color="light"
	size="xs"
	wrapperClasses="border rounded-md"
	on:click={() => {
		manualDates[selectedManualDate].setMinMax()
		dispatch('loadJobs')
	}}
	dropdownItems={[
		...manualDates.map((d, i) => ({
			label: d.label,
			onClick: () => {
				selectedManualDate = i
				d.setMinMax()
				dispatch('loadJobs')
			}
		}))
	]}
>
	<div class="flex flex-row items-center gap-2">
		<RefreshCw size={14} class={loading ? 'animate-spin' : ''} />
		{manualDates[selectedManualDate].label}
	</div>
</Button>
