<script lang="ts">
	import { RefreshCw } from 'lucide-svelte'
	import { Button } from '../common'
	import { createEventDispatcher } from 'svelte'

	export let minTs: string | undefined
	export let maxTs: string | undefined
	export let loading: boolean = false
	export let selectedManualDate = 0

	export function computeMinMax(): { minTs: string; maxTs: string | undefined } | undefined {
		return manualDates[selectedManualDate].computeMinMax()
	}

	export function resetChoice() {
		selectedManualDate = 0
	}

	function computeMinMaxInc(inc: number) {
		let minTs = new Date(new Date().getTime() - inc).toISOString()
		let maxTs = undefined
		return { minTs, maxTs }
	}

	const manualDates: {
		label: string
		computeMinMax: () => { minTs: string; maxTs: string | undefined } | undefined
	}[] = [
		{
			label: 'Last 1000 runs',
			computeMinMax: () => {
				return undefined
			}
		},
		{
			label: 'Within 30 seconds',
			computeMinMax: () => computeMinMaxInc(30 * 1000)
		},
		{
			label: 'Within last minute',
			computeMinMax: () => computeMinMaxInc(1 * 60 * 1000)
		},
		{
			label: 'Within last 5 minutes',
			computeMinMax: () => computeMinMaxInc(5 * 60 * 1000)
		},
		{
			label: 'Within last 30 minutes',
			computeMinMax: () => computeMinMaxInc(30 * 60 * 1000)
		},
		{
			label: 'Within last 24 hours',
			computeMinMax: () => computeMinMaxInc(24 * 60 * 60 * 1000)
		},
		{
			label: 'Within last 7 days',
			computeMinMax: () => computeMinMaxInc(7 * 24 * 60 * 60 * 1000)
		},
		{
			label: 'Within last month',
			computeMinMax: () => computeMinMaxInc(30 * 24 * 60 * 60 * 1000)
		}
	]

	const dispatch = createEventDispatcher()
</script>

<Button
	color="light"
	size="xs"
	variant="border"
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
