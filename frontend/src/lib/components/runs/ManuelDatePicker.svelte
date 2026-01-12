<script lang="ts">
	import { RefreshCw } from 'lucide-svelte'
	import { Button } from '../common'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		minTs: string | null
		maxTs: string | null
		loading?: boolean
		selectedManualDate?: number
		loadText?: string | undefined
		serviceLogsChoices?: boolean
		numberOfLastJobsToFetch?: number
	}

	let {
		minTs = $bindable(),
		maxTs = $bindable(),
		loading = false,
		selectedManualDate = $bindable(0),
		loadText = undefined,
		serviceLogsChoices = false,
		numberOfLastJobsToFetch = 1000
	}: Props = $props()

	export function computeMinMax(): { minTs: string; maxTs: string | null } | undefined {
		return manualDates[selectedManualDate].computeMinMax()
	}

	export function resetChoice() {
		selectedManualDate = 0
	}

	function computeMinMaxInc(inc: number) {
		let minTs = new Date(new Date().getTime() - inc).toISOString()
		let maxTs = null
		return { minTs, maxTs }
	}

	const fixedManualDates: {
		label: string
		computeMinMax: () => { minTs: string; maxTs: string | null } | undefined
	}[] = [
		...(!serviceLogsChoices
			? [
					{
						label: 'Within 30 seconds',
						computeMinMax: () => computeMinMaxInc(30 * 1000)
					},
					{
						label: 'Within last minute',
						computeMinMax: () => computeMinMaxInc(60 * 1000)
					}
				]
			: []),
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

	let manualDates = $derived([
		{
			label: loadText ?? `Last ${numberOfLastJobsToFetch} runs`,
			computeMinMax: () => {
				return undefined
			}
		},
		...fixedManualDates
	])

	const dispatch = createEventDispatcher()
</script>

<Button
	unifiedSize="md"
	variant="default"
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
					minTs = null
					maxTs = null
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
