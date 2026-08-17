<script module lang="ts">
	export {
		type Timeframe,
		buildManualTimeframe,
		serviceLogsTimeframes,
		runsTimeframes
	} from './timeframes'
	import { type Timeframe, buildManualTimeframe } from './timeframes'

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

	export function useSyncedTimeframe(
		timeframes: Timeframe[],
		getter: () => { minTs?: string | null; maxTs?: string | null; timeframe?: string | null },
		setter: (v: { minTs?: string | null; maxTs?: string | null; timeframe?: string | null }) => void
	) {
		const val = $derived.by(() => {
			const v = getter()
			if (v.minTs || v.maxTs) {
				return buildManualTimeframe(v.minTs ?? null, v.maxTs ?? null)
			} else if (v.timeframe) {
				const tf = timeframes.find((tf) => tf.label === v.timeframe)
				if (tf) return { ...tf }
			}
			return timeframes[0]
		})
		return {
			get val() {
				return val
			},
			set val(v: Timeframe) {
				if (v.type === 'manual') {
					setter({ minTs: v.minTs, maxTs: v.maxTs, timeframe: null })
				} else {
					setter({ minTs: null, maxTs: null, timeframe: v.label })
				}
			}
		}
	}
</script>

<script lang="ts">
	import { CalendarIcon, RefreshCw } from 'lucide-svelte'
	import { Button } from '../common'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Popover from '../meltComponents/Popover.svelte'
	import { watch } from 'runed'
	import { page } from '$app/state'
	import InlineCalendarInput, {
		fromCalendarDate,
		toCalendarDate
	} from '../common/InlineCalendarInput.svelte'

	interface Props {
		loading?: boolean
		items: Timeframe[]
		value: Timeframe
		wrapperClasses?: string
		onClick?: () => void
	}

	let { loading = false, onClick, items, value = $bindable(), wrapperClasses }: Props = $props()

	let isOpen = $state(false)

	// The two-calendar desktop popover needs ~780px (two min-w-9 grids + presets +
	// popover padding); below that it overflows. Under 800px use a single calendar
	// with a Start/End toggle, which keeps each bound's date + time inputs.
	const TWO_CALENDAR_MIN_WIDTH = 800
	let innerWidth = $state<number | undefined>(undefined)
	const isSmall = $derived(innerWidth != undefined && innerWidth < TWO_CALENDAR_MIN_WIDTH)
	let smallBound = $state<'start' | 'end'>('start')

	function onManualInput(input: { minTs?: string | null; maxTs?: string | null }) {
		if (value.type !== 'manual')
			value = buildManualTimeframe(input.minTs ?? null, input.maxTs ?? null)
		else {
			value = buildManualTimeframe(
				'minTs' in input ? (input.minTs ?? null) : value.minTs,
				'maxTs' in input ? (input.maxTs ?? null) : value.maxTs
			)
		}
		if (value.type == 'manual' && value.minTs == null && value.maxTs == null) {
			value = { ...items[0] }
		}
	}
</script>

<svelte:window bind:innerWidth />

{#snippet presetButtons()}
	{#each items as item (item.label)}
		<Button
			onClick={() => (value = { ...item })}
			variant="subtle"
			unifiedSize="md"
			selected={value.label === item.label}
			btnClasses="justify-start text-nowrap"
		>
			{item.label}
		</Button>
	{/each}
{/snippet}

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
	{#if value.type === 'manual'}
		<Button
			btnClasses="!rounded-none border-l-0"
			unifiedSize="md"
			onClick={() => (value = { ...items[0] })}
		>
			Reset
		</Button>
	{/if}
	<Popover
		enableFlyTransition
		placement="bottom-end"
		contentClasses={isSmall ? 'overflow-y-auto' : ''}
		bind:isOpen
	>
		{#snippet trigger()}
			<Button
				unifiedSize="md"
				iconOnly
				btnClasses="!rounded-l-none border-l-0"
				endIcon={{ icon: CalendarIcon }}
			/>
		{/snippet}
		{#snippet content()}
			{@const range = {
				end: toCalendarDate(
					value.type === 'manual' && value.maxTs ? new Date(value.maxTs) : undefined
				),
				start: toCalendarDate(
					value.type === 'manual' && value.minTs ? new Date(value.minTs) : undefined
				)
			}}
			{#if isSmall}
				<div class="flex flex-col divide-y max-w-[calc(100vw-2rem)]">
					<div class="flex flex-row flex-wrap gap-1 p-2">
						{@render presetButtons()}
					</div>
					<div class="flex flex-col gap-2 p-2">
						<ToggleButtonGroup bind:selected={smallBound} class="w-full">
							{#snippet children({ item })}
								<ToggleButton value="start" label="Start" {item} />
								<ToggleButton value="end" label="End" {item} />
							{/snippet}
						</ToggleButtonGroup>
						<InlineCalendarInput
							class="w-full"
							infiniteRange
							portalSelects
							mode="range"
							onClickBehavior={smallBound === 'end' ? 'set-end' : 'set-start'}
							bind:value={
								() => range,
								(v) =>
									onManualInput(
										smallBound === 'end'
											? { maxTs: fromCalendarDate(v.end)?.toISOString() ?? null }
											: { minTs: fromCalendarDate(v.start)?.toISOString() ?? null }
									)
							}
						/>
					</div>
				</div>
			{:else}
				<div class="flex divide-x">
					<div class="flex flex-col p-2">
						{@render presetButtons()}
					</div>
					<InlineCalendarInput
						class="p-4 max-w-[18rem]"
						infiniteRange
						mode="range"
						onClickBehavior="set-start"
						bind:value={
							() => range,
							(v) => onManualInput({ minTs: fromCalendarDate(v.start)?.toISOString() ?? null })
						}
					/>
					<InlineCalendarInput
						class="p-4 max-w-[18rem]"
						infiniteRange
						mode="range"
						onClickBehavior="set-end"
						bind:value={
							() => range,
							(v) => onManualInput({ maxTs: fromCalendarDate(v.end)?.toISOString() ?? null })
						}
					/>
				</div>
			{/if}
		{/snippet}
	</Popover>
</div>
