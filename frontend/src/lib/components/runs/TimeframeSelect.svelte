<script module lang="ts">
	function computeMinMaxInc(inc: number) {
		let minTs = new Date(new Date().getTime() - inc).toISOString()
		let maxTs = new Date().toISOString()
		return { minTs, maxTs }
	}

	export type Timeframe =
		| {
				label: string
				computeMinMax: () => { minTs: string | null; maxTs: string | null }
				type: 'dynamic'
		  }
		| {
				label: string
				computeMinMax: () => { minTs: string | null; maxTs: string | null }
				minTs: string | null
				maxTs: string | null
				type: 'manual'
		  }

	export function buildManualTimeframe(minTs: string | null, maxTs: string | null): Timeframe {
		return {
			label: formatDateRange(minTs ?? undefined, maxTs ?? undefined),
			minTs,
			maxTs,
			type: 'manual',
			computeMinMax: () => ({ minTs, maxTs })
		}
	}

	export const serviceLogsTimeframes: Timeframe[] = [
		{ label: '1000 last service logs', computeMinMax: () => ({ minTs: null, maxTs: null }) },
		{ label: 'Within last 5 minutes', computeMinMax: () => computeMinMaxInc(5 * 60 * 1000) },
		{ label: 'Within last 30 minutes', computeMinMax: () => computeMinMaxInc(30 * 60 * 1000) },
		{ label: 'Within last 24 hours', computeMinMax: () => computeMinMaxInc(24 * 60 * 60 * 1000) },
		{ label: 'Within last 7 days', computeMinMax: () => computeMinMaxInc(7 * 24 * 60 * 60 * 1000) },
		{ label: 'Within last month', computeMinMax: () => computeMinMaxInc(30 * 24 * 60 * 60 * 1000) }
	].map((item) => ({ ...item, type: 'dynamic' }))

	export const runsTimeframes: Timeframe[] = [
		{ label: 'Latest runs', computeMinMax: () => ({ minTs: null, maxTs: null }) },
		{ label: 'Within 30 seconds', computeMinMax: () => computeMinMaxInc(30 * 1000) },
		{ label: 'Within last minute', computeMinMax: () => computeMinMaxInc(60 * 1000) },
		{ label: 'Within last 5 minutes', computeMinMax: () => computeMinMaxInc(5 * 60 * 1000) },
		{ label: 'Within last 30 minutes', computeMinMax: () => computeMinMaxInc(30 * 60 * 1000) },
		{ label: 'Within last 24 hours', computeMinMax: () => computeMinMaxInc(24 * 60 * 60 * 1000) },
		{ label: 'Within last 7 days', computeMinMax: () => computeMinMaxInc(7 * 24 * 60 * 60 * 1000) },
		{ label: 'Within last month', computeMinMax: () => computeMinMaxInc(30 * 24 * 60 * 60 * 1000) }
	].map((item) => ({ ...item, type: 'dynamic' }))

	export function useUrlSyncedTimeframe(timeframes: Timeframe[]) {
		let obj = $state({ timeframe: timeframes[0] })
		let timeframe = $derived(obj.timeframe)
		watch(
			() => [page, timeframe],
			() => {
				const url = new URL(page.url)

				if (timeframe.type === 'manual' && timeframe.minTs)
					url.searchParams.set('min_ts', timeframe.minTs)
				else url.searchParams.delete('min_ts')

				if (timeframe.type === 'manual' && timeframe.maxTs)
					url.searchParams.set('max_ts', timeframe.maxTs)
				else url.searchParams.delete('max_ts')

				if (timeframe.type === 'dynamic' && timeframe.label !== timeframes[0].label)
					url.searchParams.set('timeframe', timeframe.label)
				else url.searchParams.delete('timeframe')

				history.replaceState(null, '', url)
			}
		)

		if (page.url.searchParams.get('min_ts') || page.url.searchParams.get('max_ts')) {
			obj.timeframe = buildManualTimeframe(
				page.url.searchParams.get('min_ts') || null,
				page.url.searchParams.get('max_ts') || null
			)
		} else {
			const tfLabel = page.url.searchParams.get('timeframe')
			const tf = timeframes.find((tf) => tf.label === tfLabel)
			if (tf) obj.timeframe = { ...tf }
		}

		return obj
	}
</script>

<script lang="ts">
	import { CalendarIcon, RefreshCw } from 'lucide-svelte'
	import { Button } from '../common'
	import Popover from '../meltComponents/Popover.svelte'
	import DateTimeInput from '../DateTimeInput.svelte'
	import Label from '../Label.svelte'
	import { formatDateRange } from '$lib/utils'
	import { watch } from 'runed'
	import { page } from '$app/state'

	interface Props {
		loading?: boolean
		items: Timeframe[]
		value: Timeframe
		wrapperClasses?: string
		onClick?: () => void
	}

	let { loading = false, onClick, items, value = $bindable(), wrapperClasses }: Props = $props()

	let isOpen = $state(false)

	function onManualInput(input: { minTs?: string | null; maxTs?: string | null }) {
		if (value.type !== 'manual')
			value = buildManualTimeframe(input.minTs ?? null, input.maxTs ?? null)
		else
			value = buildManualTimeframe(
				'minTs' in input ? (input.minTs ?? null) : value.minTs,
				'maxTs' in input ? (input.maxTs ?? null) : value.maxTs
			)
		if (value.type == 'manual' && value.minTs == null && value.maxTs == null) {
			value = { ...items[0] }
		}
	}
</script>

<div class="relative flex {wrapperClasses}">
	<Button
		unifiedSize="md"
		wrapperClasses="flex-1"
		btnClasses="!rounded-r-none whitespace-nowrap"
		onClick={() => onClick?.()}
	>
		<div class="flex flex-row items-center gap-2">
			<RefreshCw size={14} class={loading ? 'animate-spin' : ''} />
			{value.label}
		</div>
	</Button>
	<Popover enableFlyTransition bind:isOpen>
		{#snippet trigger()}
			<Button
				unifiedSize="md"
				iconOnly
				btnClasses="!rounded-l-none border-l-0"
				endIcon={{ icon: CalendarIcon }}
			/>
		{/snippet}
		{#snippet content()}
			<div class="flex flex-col">
				{#each items as item}
					<Button
						onClick={() => ((value = { ...item }), (isOpen = false))}
						variant="subtle"
						unifiedSize="md"
						btnClasses="justify-start"
					>
						{item.label}
					</Button>
				{/each}
				<div class="border-b"></div>
				<div class="px-4 py-2 flex flex-col gap-2">
					<Label label="From">
						<DateTimeInput
							clearable
							bind:value={
								() => (value.type === 'manual' ? value.minTs : undefined),
								(v) => onManualInput({ minTs: v ?? null })
							}
						/>
					</Label>
					<Label label="To">
						<DateTimeInput
							clearable
							bind:value={
								() => (value.type === 'manual' ? value.maxTs : undefined),
								(v) => onManualInput({ maxTs: v ?? null })
							}
						/>
					</Label>
				</div>
			</div>
		{/snippet}
	</Popover>
</div>
