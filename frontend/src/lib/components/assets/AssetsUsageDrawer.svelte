<script lang="ts">
	import type { AssetKind, AssetUsageAccessType, AssetUsageKind } from '$lib/gen'
	import { Drawer, DrawerContent } from '../common'
	import RowIcon from '../common/table/RowIcon.svelte'
	import { formatAssetAccessType, getAssetUsagePageUri } from './lib'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'

	let usagesDrawerData:
		| {
				path: string
				kind: AssetKind
				usages: {
					path: string
					kind: AssetUsageKind
					access_type?: AssetUsageAccessType
					created_at?: string
				}[]
		  }
		| undefined = $state()

	export function open(data: typeof usagesDrawerData) {
		usagesDrawerData = data
	}

	// const jobStatusColor = {
	// 	running: 'text-yellow-500',
	// 	success: 'text-green-500',
	// 	failure: 'text-red-500',
	// 	canceled: 'text-red-500'
	// }

	let staticUsages = $derived(usagesDrawerData?.usages.filter((u) => u.kind !== 'job') ?? [])
	let runtimeUsages = $derived(usagesDrawerData?.usages.filter((u) => u.kind === 'job') ?? [])
</script>

<Drawer
	open={usagesDrawerData !== undefined}
	size="900px"
	on:close={() => (usagesDrawerData = undefined)}
>
	<DrawerContent title="Asset usage" on:close={() => (usagesDrawerData = undefined)}>
		<div class="flex flex-col gap-6">
			<!-- Static/Declared Usages -->
			<section>
				<h3 class="text-sm font-semibold mb-2">Scripts & Flows</h3>
				<ul class="flex flex-col border rounded-md divide-y">
					{#each staticUsages as u}
						{@const accessType = formatAssetAccessType(u.access_type)}
						<li>
							<a
								href={getAssetUsagePageUri(u)}
								aria-label={`${u.kind}/${u.path}`}
								class="text-xs text-primary font-normal flex items-center py-3 px-4 gap-2 hover:bg-surface-hover cursor-pointer"
							>
								<RowIcon kind={u.kind as Exclude<typeof u.kind, 'job'>} />
								<div class="flex flex-col justify-center flex-1">
									<span class="font-semibold text-emphasis">{u.path}</span>
									<span class="text-2xs text-secondary">{u.kind}</span>
								</div>
								{@render badge(displayDate(u.created_at), 'Asset detection time')}
								{@render badge(accessType)}
							</a>
						</li>
					{/each}
				</ul>
			</section>

			<!-- Runtime Job Usage -->
			<section>
				<h3 class="text-sm font-semibold mb-2 flex justify-between items-center">
					Latest Job Executions
				</h3>
				<Alert
					type="info"
					class="mb-2"
					title="Assets are processed asynchronously after job completion"
				>
					It may take a few minutes for jobs to show up here.
				</Alert>

				<div
					class={twMerge(
						'flex flex-col border rounded-md divide-y',
						!runtimeUsages.length ? 'hidden' : ''
					)}
				>
					{#each runtimeUsages as u}
						{@const accessType = formatAssetAccessType(u.access_type)}
						<a
							href={`/run/${u.path}?workspace=${$workspaceStore}`}
							class="text-xs text-primary font-normal flex items-center py-3 px-4 gap-2 hover:bg-surface-hover cursor-pointer"
						>
							<div class="flex flex-col justify-center flex-1">
								<span class="text-emphasis">{u.path}</span>
							</div>
							{@render badge(displayDate(u.created_at), 'Asset detection time')}
							{@render badge(accessType)}
						</a>
					{/each}
				</div>
			</section>
		</div>
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
