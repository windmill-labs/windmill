<script lang="ts">
	import type { AssetUsageAccessType, AssetUsageDetectionKind, AssetUsageKind } from '$lib/gen'
	import { Drawer, DrawerContent } from '../common'
	import RowIcon from '../common/table/RowIcon.svelte'
	import { formatAssetAccessType, getAssetUsagePageUri } from './lib'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'

	let usagesDrawerData:
		| {
				usages: {
					path: string
					kind: AssetUsageKind
					access_type?: AssetUsageAccessType
					detection_kinds?: AssetUsageDetectionKind[]
				}[]
		  }
		| undefined = $state()

	export function open(data: typeof usagesDrawerData) {
		usagesDrawerData = data
	}

	$inspect('usagesDrawerData', usagesDrawerData)
</script>

<Drawer
	open={usagesDrawerData !== undefined}
	size="900px"
	on:close={() => (usagesDrawerData = undefined)}
>
	<DrawerContent title="Asset usage" on:close={() => (usagesDrawerData = undefined)}>
		<ul class="flex flex-col border rounded-md divide-y">
			{#each usagesDrawerData?.usages ?? [] as u}
				{@const accessType = formatAssetAccessType(u.access_type)}
				<li>
					<a
						href={getAssetUsagePageUri(u)}
						aria-label={`${u.kind}/${u.path}`}
						class="text-xs text-primary font-normal flex items-center py-3 px-4 gap-2 hover:bg-surface-hover cursor-pointer"
					>
						<RowIcon kind={u.kind} />
						<div class="flex flex-col justify-center flex-1">
							<span class="font-semibold text-emphasis">{u.path}</span>
							<span class="text-2xs text-secondary">{u.kind}</span>
						</div>
						{@render badge(
							u.detection_kinds?.includes('runtime') ? 'Runtime' : undefined,
							'The asset was used as a job input'
						)}
						{@render badge(
							u.detection_kinds?.includes('static') ? 'Static' : undefined,
							'The asset was detected while statically analyzing the code'
						)}
						{@render badge(accessType)}
					</a>
				</li>
			{/each}
		</ul>
	</DrawerContent>
</Drawer>

{#snippet badge(text: string | undefined, tooltip?: string)}
	{#if text}
		<Tooltip disablePopup={!tooltip}>
			<div
				class={twMerge(
					'text-xs bg-surface font-normal border text-primary min-w-12 p-1 text-center rounded-md'
				)}
			>
				{text}
			</div>
			<svelte:fragment slot="text">
				{#if tooltip}
					{tooltip}
				{/if}
			</svelte:fragment>
		</Tooltip>
	{/if}
{/snippet}
