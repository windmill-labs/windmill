<script lang="ts">
	import type { RunStartedBy, RunnableType } from '$lib/gen'
	import { ChevronDown } from 'lucide-svelte'
	import { Button } from './common'
	import Checkbox from './common/checkbox/Checkbox.svelte'
	import { Popover } from './meltComponents'
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
	}

	let { selected, runnableType, onChange }: Props = $props()

	let available = $derived(availableCategories(runnableType))
	let isDefault = $derived(isDefaultSelection(available.filter((c) => selected.includes(c))))

	const sections = [
		{ title: 'Started on their own', categories: TOP_LEVEL_CATEGORIES },
		{ title: 'Started inside another run', categories: INSIDE_CATEGORIES }
	]

	function toggle(category: RunStartedBy, on: boolean) {
		const others = selected.filter((c) => c !== category)
		onChange(on ? [...others, category] : others)
		logRunHistoryFilter(`${category}:${on ? 'on' : 'off'}`)
	}
</script>

<Popover
	class="w-fit"
	usePointerDownOutside
	closeOnOtherPopoverOpen
	on:click={(e) => {
		e.stopPropagation()
	}}
	floatingConfig={{ placement: 'bottom-end' }}
>
	{#snippet trigger()}
		<Button
			unifiedSize="sm"
			variant="default"
			selected={!isDefault}
			endIcon={{ icon: ChevronDown }}
			nonCaptureEvent
			title="Kinds of runs listed"
		>
			{selectionLabel(selected, available)}
		</Button>
	{/snippet}
	{#snippet content()}
		<div class="p-3 w-[440px] max-w-[90vw] flex flex-col gap-2">
			<div class="grid grid-cols-2 gap-x-6">
				{#each sections as section (section.title)}
					<div class="flex flex-col gap-1">
						<div class="text-2xs font-semibold text-secondary pb-0.5">{section.title}</div>
						{#each section.categories.filter((c) => available.includes(c)) as category (category)}
							<label class="flex items-start gap-2 text-xs text-primary cursor-pointer py-0.5">
								<!-- Toggled on click: the popover cancels the click's default, so no
								     change event would follow. -->
								<Checkbox
									class="mt-0.5"
									checked={selected.includes(category)}
									onClick={() => toggle(category, !selected.includes(category))}
								/>
								<span class="flex flex-col">
									{CATEGORY_LABELS[category].label}
									{#if CATEGORY_LABELS[category].hint}
										<span class="text-2xs text-tertiary">{CATEGORY_LABELS[category].hint}</span>
									{/if}
								</span>
							</label>
						{/each}
					</div>
				{/each}
			</div>
			<div class="flex items-center justify-between gap-2 border-t border-border-light pt-2">
				<span class="text-2xs text-tertiary">Retries are listed under their first attempt.</span>
				<Button
					variant="subtle"
					unifiedSize="xs"
					disabled={isDefault}
					onClick={() => {
						onChange([...TOP_LEVEL_CATEGORIES])
						logRunHistoryFilter('reset')
					}}
				>
					Reset to default
				</Button>
			</div>
		</div>
	{/snippet}
</Popover>
