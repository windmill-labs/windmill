<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { ListFilterPlus } from 'lucide-svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import type { Job } from '$lib/gen'
	import type { Item } from '$lib/utils'

	interface Props {
		job: Job
		activeLabel: string | null
		onFilterByLabel: (label: string) => void
		labelWidth: number
	}

	let { job, activeLabel, onFilterByLabel, labelWidth }: Props = $props()

	const GAP = 4
	const LABEL_MAX_WIDTH = 112
	const MORE_LABEL_WIDTH = 30

	const labels = $derived(job && Array.isArray(job?.['labels']) ? (job['labels'] as string[]) : [])

	const labelSplit = $derived.by(() => {
		if (!labels || labels.length === 0 || labelWidth <= 0) {
			return { visibleLabels: [], hiddenLabels: [] }
		}

		if (labels.length === 1) {
			return { visibleLabels: labels, hiddenLabels: [] }
		}

		let currentWidth = 0
		const visible: string[] = []
		const hidden: string[] = []
		const margin = 20

		for (let i = 0; i < labels.length; i++) {
			const label = labels[i]

			// Check if we need to reserve space for overflow badge
			const needsOverflowBadge = i < labels.length - 1
			const remainingWidth = labelWidth - currentWidth
			const requiredWidth =
				LABEL_MAX_WIDTH + (needsOverflowBadge ? MORE_LABEL_WIDTH + GAP : 0) + margin

			if (remainingWidth >= requiredWidth || visible.length === 0) {
				visible.push(label)
				currentWidth += LABEL_MAX_WIDTH + GAP
			} else {
				hidden.push(...labels.slice(i))
				break
			}
		}

		return { visibleLabels: visible, hiddenLabels: hidden }
	})

	const visibleLabels = $derived(labelSplit.visibleLabels || [])
	const hiddenLabels = $derived(labelSplit.hiddenLabels || [])

	const dropdownItems = $derived(
		hiddenLabels.map(
			(label): Item => ({
				displayName: label,
				action: () => onFilterByLabel(label),
				icon: ListFilterPlus
			})
		)
	)
</script>

{#if labels && labels.length > 0}
	<div class="flex flex-row items-center" style="gap: {GAP}px">
		{#each visibleLabels as label}
			<Tooltip openDelay={500} placement="bottom">
				<button
					class={twMerge(
						activeLabel == label ? 'bg-blue-50 dark:bg-blue-900/50' : '',
						'flex flex-row items-center px-2 group py-1 rounded-md bg-surface-secondary hover:bg-surface'
					)}
					style="gap: {GAP}px; width: {LABEL_MAX_WIDTH}px"
					onclick={() => {
						onFilterByLabel(label)
					}}
				>
					<span class="truncate text-2xs font-normal">{label}</span>
					<ListFilterPlus size={12} class="shrink-0 text-gray-300 group-hover:text-primary" />
				</button>
				{#snippet text()}
					{`Filter by label: ${label}`}
				{/snippet}
			</Tooltip>
		{/each}

		{#if hiddenLabels.length > 0}
			<DropdownV2 placement="bottom-start" items={dropdownItems} customWidth={224}>
				{#snippet buttonReplacement()}
					<button
						class="flex flex-row items-center justify-center px-2 py-1 text-2xs font-semibold hover:bg-surface bg-surface-secondary text-secondary rounded-md"
						style="gap: {GAP}px; width: {MORE_LABEL_WIDTH}px"
					>
						+{hiddenLabels.length}
					</button>
				{/snippet}
			</DropdownV2>
		{/if}
	</div>
{:else}
	<span class="text-2xs text-secondary">No labels</span>
{/if}
