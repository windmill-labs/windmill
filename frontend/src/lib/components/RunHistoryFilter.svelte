<script lang="ts">
	import type { RunStartedBy, RunnableType } from '$lib/gen'
	import type { Item } from '$lib/utils'
	import { ChevronDown, ListFilter } from 'lucide-svelte'
	import { Button } from './common'
	import DropdownV2 from './DropdownV2.svelte'
	import {
		availableCategories,
		CATEGORY_LABELS,
		INSIDE_CATEGORIES,
		isDefaultSelection,
		logRunHistoryFilter,
		selectionLabel,
		TOP_LEVEL_CATEGORIES
	} from './runHistory'

	interface Props {
		selected: RunStartedBy[]
		runnableType: RunnableType | undefined
		onChange: (selected: RunStartedBy[]) => void
		/** An icon instead of the labelled button, for a narrow panel. */
		compact?: boolean
	}

	let { selected, runnableType, onChange, compact = false }: Props = $props()

	let available = $derived(availableCategories(runnableType))
	let isDefault = $derived(isDefaultSelection(available.filter((c) => selected.includes(c))))
	let label = $derived(selectionLabel(selected, available))

	function toggle(category: RunStartedBy) {
		const on = !selected.includes(category)
		const others = selected.filter((c) => c !== category)
		onChange(on ? [...others, category] : others)
		logRunHistoryFilter(`${category}:${on ? 'on' : 'off'}`)
	}

	function section(title: string, categories: RunStartedBy[], first: boolean): Item[] {
		return categories
			.filter((c) => available.includes(c))
			.map((category, i) => ({
				displayName: CATEGORY_LABELS[category].label,
				description: CATEGORY_LABELS[category].hint,
				toggle: selected.includes(category),
				sectionTitle: i === 0 ? title : undefined,
				separatorTop: i === 0 && !first,
				action: () => toggle(category)
			}))
	}

	const items = (): Item[] => [
		...section('Started on their own', TOP_LEVEL_CATEGORIES, true),
		...section('Started inside another run', INSIDE_CATEGORIES, false),
		{
			displayName: 'Reset to default',
			description: 'Retries are listed under their first attempt.',
			separatorTop: true,
			disabled: isDefault,
			action: () => {
				onChange([...TOP_LEVEL_CATEGORIES])
				logRunHistoryFilter('reset')
			}
		}
	]
</script>

<DropdownV2
	{items}
	closeOnItemClick={false}
	usePointerDownOutside
	fixedHeight={false}
	customWidth={260}
>
	{#snippet buttonReplacement()}
		<!-- Fixed width: the label changes with every click, and a button that resized would
		     move the open menu anchored to it. -->
		{#if compact}
			<Button
				unifiedSize="sm"
				variant="default"
				selected={!isDefault}
				startIcon={{ icon: ListFilter }}
				iconOnly
				nonCaptureEvent
				title={`Kinds of runs listed: ${label}`}
			/>
		{:else}
			<Button
				unifiedSize="sm"
				variant="default"
				selected={!isDefault}
				endIcon={{ icon: ChevronDown }}
				nonCaptureEvent
				btnClasses="w-40 justify-between"
				title={`Kinds of runs listed: ${label}`}
			>
				<span class="truncate">{label}</span>
			</Button>
		{/if}
	{/snippet}
</DropdownV2>
