<script lang="ts">
	import { RefreshCw } from 'lucide-svelte'
	import { Button } from '../common'
	import { createEventDispatcher } from 'svelte'

	export let minTs: string | undefined
	export let maxTs: string | undefined
	export let loading: boolean = false
	export let selectedManualDate = 0
	export let loadText: string | undefined = undefined
	export let serviceLogsChoices: boolean = false

	export function computeMinMax(): { minTs: string; maxTs: string } | undefined {
		return manualDates[selectedManualDate].computeMinMax()
	}

	function computeMinMaxInc(inc: number) {
		let minTs = new Date(new Date().getTime() - inc).toISOString()
		let maxTs = new Date().toISOString()
		return { minTs, maxTs }
	}

	const manualDates: {
		label: string
		computeMinMax: () => { minTs: string; maxTs: string } | undefined
	}[] = [
		{
			label: loadText ?? 'Last 1000 runs',
			computeMinMax: () => {
				return undefined
			}
		},
		...(!serviceLogsChoices
			? [
					{
						label: 'Within 30 seconds',
						computeMinMax: () => {
							return computeMinMaxInc(30 * 1000)
						}
					},
					{
						label: 'Within last minute',
						computeMinMax: () => {
							return computeMinMaxInc(60 * 1000)
						}
					}
			  ]
			: []),
		{
			label: 'Within last 5 minutes',
			computeMinMax: () => {
				return computeMinMaxInc(5 * 60 * 1000)
			}
		},
		{
			label: 'Within last 30 minutes',
			computeMinMax: () => {
				return computeMinMaxInc(30 * 60 * 1000)
			}
		},
		...(serviceLogsChoices
			? [
					{
						label: 'Within last hour',
						computeMinMax: () => {
							return computeMinMaxInc(60 * 60 * 1000)
						}
					},
					{
						label: 'Within last 6 hours',
						computeMinMax: () => {
							return computeMinMaxInc(6 * 60 * 60 * 1000)
						}
					},
					{
						label: 'Within last 12 hours',
						computeMinMax: () => {
							return computeMinMaxInc(12 * 60 * 60 * 1000)
						}
					}
			  ]
			: [
					{
						label: 'Within last 24 hours',
						computeMinMax: () => {
							return computeMinMaxInc(24 * 60 * 60 * 1000)
						}
					},
					{
						label: 'Within last 7 days',
						computeMinMax: () => {
							return computeMinMaxInc(7 * 24 * 60 * 60 * 1000)
						}
					},
					{
						label: 'Within last month',
						computeMinMax: () => {
							return computeMinMaxInc(30 * 24 * 60 * 60 * 1000)
						}
					}
			  ])
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
		dispatch('loadJobs', { minTs, maxTs })
	}}
	dropdownItems={[
		...manualDates.map((d, i) => ({
			label: d.label,
			onClick: (e) => {
				e.preventDefault()
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
