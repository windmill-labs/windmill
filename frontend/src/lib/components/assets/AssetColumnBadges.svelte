<script lang="ts">
	import { type AssetUsageAccessType } from 'windmill-utils-internal/dist/gen/types.gen'
	import { formatAssetAccessType } from './lib'
	import Tooltip from '../meltComponents/Tooltip.svelte'

	type Props = {
		columns: Record<string, AssetUsageAccessType>
		disableTooltip?: boolean
	}
	let { columns, disableTooltip }: Props = $props()
</script>

<div class="flex gap-1 flex-wrap mt-0.5">
	{#each Object.entries(columns) as [columnName, accessType]}
		{@const accessType2 = formatAssetAccessType(accessType)}
		{#snippet badge()}
			<div
				class="text-xs text-secondary border rounded-md px-1 bg-surface-tertiary dark:bg-surface-secondary"
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
