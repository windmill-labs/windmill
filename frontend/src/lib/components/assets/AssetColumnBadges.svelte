<script lang="ts">
	import { type AssetUsageAccessType } from '$lib/gen'
	import { formatAssetAccessType } from './lib'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'

	type Props = {
		columns: Record<string, AssetUsageAccessType> | undefined
		disableTooltip?: boolean
		badgeClasses?: string
		disableWrap?: boolean
	}
	let { columns, disableTooltip, badgeClasses, disableWrap }: Props = $props()

	let entries = $derived(columns && Object.entries(columns))
</script>

{#if entries?.length}
	<div class={twMerge('flex gap-1', disableWrap ? '' : 'flex-wrap')}>
		{#each entries as [columnName, accessType]}
			{@const accessType2 = formatAssetAccessType(accessType)}
			{#snippet badge()}
				<div
					class={twMerge(
						'text-xs text-secondary border rounded-md px-1 bg-surface-tertiary dark:bg-surface-secondary',
						badgeClasses
					)}
				>
					{columnName}
				</div>
			{/snippet}
			{#if disableTooltip}
				{@render badge()}
			{:else}
				<Tooltip>
					{@render badge()}
					<svelte:fragment slot="text">
						{accessType2} access to column "{columnName}"
					</svelte:fragment>
				</Tooltip>
			{/if}
		{/each}
	</div>
{/if}
