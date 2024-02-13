<script lang="ts">
	import { RefreshCw } from 'lucide-svelte'
	import { Button } from '../common'
	import { createEventDispatcher } from 'svelte'

	export let minTs: string | undefined
	export let maxTs: string | undefined
	export let loading: boolean = false
	export let selectedManualDate = 0

	export function computeMinMax(): { minTs: string; maxTs: string } | undefined {
		return manualDates[selectedManualDate].computeMinMax()
	}

	const manualDates: {
		label: string
		computeMinMax: () => { minTs: string; maxTs: string } | undefined
	}[] = [
		{
			label: 'Last 1000 runs',
			computeMinMax: () => {
				return undefined
			}
		},
		{
			label: 'Within 30 seconds',
			computeMinMax: () => {
				let minTs = new Date(new Date().getTime() - 30 * 1000).toISOString()
				let maxTs = new Date().toISOString()
				return { minTs, maxTs }
			}
		},
		{
			label: 'Within last minute',
			computeMinMax: () => {
				let minTs = new Date(new Date().getTime() - 60 * 1000).toISOString()
				let maxTs = new Date().toISOString()
				return { minTs, maxTs }
			}
		},
		{
			label: 'Within last 5 minutes',
			computeMinMax: () => {
				let minTs = new Date(new Date().getTime() - 5 * 60 * 1000).toISOString()
				let maxTs = new Date().toISOString()
				return { minTs, maxTs }
			}
		},
		{
			label: 'Within last 30 minutes',
			computeMinMax: () => {
				let minTs = new Date(new Date().getTime() - 30 * 60 * 1000).toISOString()
				let maxTs = new Date().toISOString()
				return { minTs, maxTs }
			}
		},
		{
			label: 'Within last 24 hours',
			computeMinMax: () => {
				let minTs = new Date(new Date().getTime() - 24 * 60 * 60 * 1000).toISOString()
				let maxTs = new Date().toISOString()
				return { minTs, maxTs }
			}
		},
		{
			label: 'Within last 7 days',
			computeMinMax: () => {
				let minTs = new Date(new Date().getTime() - 7 * 24 * 60 * 60 * 1000).toISOString()
				let maxTs = new Date().toISOString()
				return { minTs, maxTs }
			}
		},
		{
			label: 'Within last month',
			computeMinMax: () => {
				let minTs = new Date(new Date().getTime() - 30 * 24 * 60 * 60 * 1000).toISOString()
				let maxTs = new Date().toISOString()
				return { minTs, maxTs }
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
		const ts = computeMinMax()
		if (ts) {
			minTs = ts.minTs
			maxTs = ts.maxTs
		}
		dispatch('loadJobs')
	}}
	dropdownItems={[
		...manualDates.map((d, i) => ({
			label: d.label,
			onClick: () => {
				selectedManualDate = i
				const ts = d.computeMinMax()
				if (ts) {
					minTs = ts.minTs
					maxTs = ts.maxTs
				} else {
					minTs = undefined
					maxTs = undefined
				}
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
