<script lang="ts">
	import type { AssetKind } from '$lib/gen'
	import { Drawer, DrawerContent } from '../common'
	import RowIcon from '../common/table/RowIcon.svelte'
	import { formatAssetAccessType, getAssetUsagePageUri, type AssetUsage } from './lib'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import Tooltip2 from '../Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'
	import { displayDate } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import { type AssetUsageAccessType } from 'windmill-utils-internal/dist/gen/types.gen'

	let usagesDrawerData:
		| {
				path: string
				kind: AssetKind
				usages: AssetUsage[]
		  }
		| undefined = $state()

	export function open(data: typeof usagesDrawerData) {
		usagesDrawerData = data
	}

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
				<h3 class="text-sm font-semibold mb-2">
					Scripts & Flows
					<Tooltip2>Assets detected statically during code analysis</Tooltip2>
				</h3>
				{@render list(staticUsages)}
			</section>

			<!-- Runtime Job Usage -->
			<section>
				<h3 class="text-sm font-semibold mb-2">
					Latest Job Executions
					<Tooltip2>
						Assets can be detected during job execution either via Windmill's SDK (Data tables, S3
						objects ...) or by being passed as inputs to a script or flow (arbitrary resources).
					</Tooltip2>
				</h3>
				<Alert
					type="info"
					class="mb-2"
					title="Assets are processed asynchronously after job completion"
				>
					It may take a few minutes for jobs to show up here.
				</Alert>

				{@render list(runtimeUsages)}
			</section>
		</div>
	</DrawerContent>
</Drawer>

{#snippet rightBadge(text: string | undefined, tooltip?: string)}
	{#if text}
		<Tooltip disablePopup={!tooltip}>
			<div class={twMerge('text-xs 	font-normal text-primary min-w-12 p-1 text-center rounded-md')}>
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

{#snippet columnBadges(columns: Record<string, AssetUsageAccessType>)}
	<div class="flex gap-1 flex-wrap">
		{#each Object.entries(columns) as [columnName, accessType]}
			{@const accessType2 = formatAssetAccessType(accessType)}
			<Tooltip>
				<div class="text-xs text-secondary border rounded-md px-1">{columnName}</div>
				<svelte:fragment slot="text">
					{accessType2} access to column "{columnName}"
				</svelte:fragment>
			</Tooltip>
		{/each}
	</div>
{/snippet}

{#snippet list(items: AssetUsage[])}
	<ul class="flex flex-col border rounded-md divide-y">
		{#each items as u}
			{@const accessType = formatAssetAccessType(u.access_type)}

			<li>
				<a
					href={getAssetUsagePageUri(u)}
					aria-label={`${u.kind}/${u.path}`}
					class="text-xs text-primary font-normal flex items-center py-3 px-4 gap-2 hover:bg-surface-hover cursor-pointer"
				>
					<RowIcon
						kind={!u.metadata?.job_kind
							? u.kind != 'job'
								? u.kind
								: 'script'
							: ((
									{
										script: 'script',
										preview: 'script',
										flow: 'flow',
										flowpreview: 'flow',
										singlestepflow: 'flow',
										flownode: 'flow'
									} as const
								)[u.metadata.job_kind] ?? 'script')}
					/>
					<div class="flex flex-col justify-center flex-1">
						<span class="font-semibold text-emphasis">
							{u.kind == 'job' ? (u.metadata?.runnable_path ?? 'Unknown job') : u.path}
						</span>
						{#if u.kind == 'job'}
							<span class="text-2xs text-secondary">{u.path}</span>
						{/if}
						{#if u.columns}
							{@render columnBadges(u.columns)}
						{/if}
					</div>
					{@render rightBadge(displayDate(u.created_at), 'Asset detection time')}
					{@render rightBadge(accessType)}
				</a>
			</li>
		{/each}
	</ul>
{/snippet}
