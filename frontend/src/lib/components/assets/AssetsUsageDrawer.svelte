<script lang="ts">
	import type {
		AssetKind,
		AssetUsageAccessType,
		AssetUsageDetectionKind,
		AssetUsageKind
	} from '$lib/gen'
	import { Drawer, DrawerContent } from '../common'
	import RowIcon from '../common/table/RowIcon.svelte'
	import { formatAssetAccessType, getAssetUsagePageUri } from './lib'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'
	import { AssetService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import Button from '../common/button/Button.svelte'
	import RefreshButton from '../common/button/RefreshButton.svelte'
	import { resource } from 'runed'
	import { StaleWhileLoading } from '$lib/svelte5Utils.svelte'

	let usagesDrawerData:
		| {
				path: string
				kind: AssetKind
				usages: {
					path: string
					kind: AssetUsageKind
					access_type?: AssetUsageAccessType
					detection_kinds?: AssetUsageDetectionKind[]
				}[]
		  }
		| undefined = $state()

	let _runtimeJobs = resource(
		[() => runtimeJobsCurrentPage, () => usagesDrawerData],
		async ([page, data]) => {
			if (!data || !data.usages.some((u) => u.detection_kinds?.includes('runtime')))
				return undefined
			return await AssetService.listAssetJobs({
				workspace: $workspaceStore!,
				assetPath: data.path,
				assetKind: data.kind,
				page,
				perPage: 20
			})
		}
	)
	const runtimeJobs = new StaleWhileLoading(() => _runtimeJobs.current)

	let runtimeJobsCurrentPage = $state(1)

	export function open(data: typeof usagesDrawerData) {
		usagesDrawerData = data
		runtimeJobsCurrentPage = 1
	}

	const jobStatusColor = {
		running: 'text-yellow-500',
		success: 'text-green-500',
		failure: 'text-red-500',
		canceled: 'text-red-500'
	}
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
			</section>

			<!-- Runtime Job Usage -->
			{#if usagesDrawerData?.usages.some((u) => u.detection_kinds?.includes('runtime'))}
				<section>
					<h3 class="text-sm font-semibold mb-2 flex justify-between items-center">
						Recent Job Executions
						<RefreshButton loading={_runtimeJobs.loading} onClick={() => _runtimeJobs.refetch()} />
					</h3>

					{#if !runtimeJobs.current && _runtimeJobs.loading}
						<div class="flex items-center justify-center py-8 text-sm text-secondary">
							Loading jobs...
						</div>
					{:else if runtimeJobs.current}
						<div class="flex flex-col border rounded-md divide-y">
							{#each runtimeJobs.current.jobs as job}
								<a
									href={`/run/${job.id}?workspace=${$workspaceStore}`}
									class="text-xs text-primary font-normal flex items-center py-3 px-4 gap-3 hover:bg-surface-hover cursor-pointer"
								>
								<span class="mr-1 text-lg {jobStatusColor[job.status || 'running'] || jobStatusColor['running']}">•</span>
									<div class="flex flex-col justify-center flex-1">
										<div class="flex items-center gap-2">
											<span class="font-semibold text-emphasis">
												{job.runnable_path || 'Inline script'}
											</span>
										</div>
										<div class="text-2xs text-secondary flex items-center gap-2">
											<span>{job.created_by}</span>
											<span>•</span>
											<span>{displayDate(job.created_at)}</span>
										</div>
									</div>
									{@render badge(job.id, 'Job ID')}
								</a>
							{/each}
						</div>

						<!-- Pagination -->
						{#if runtimeJobs.current.total > runtimeJobs.current.per_page}
							<div class="flex items-center justify-between mt-3 text-sm">
								<span class="text-secondary">
									Showing {(runtimeJobs.current.page - 1) * runtimeJobs.current.per_page + 1} -
									{Math.min(
										runtimeJobs.current.page * runtimeJobs.current.per_page,
										runtimeJobs.current.total
									)} of {runtimeJobs.current.total}
									jobs
								</span>
								<div class="flex gap-2">
									<Button
										size="xs"
										color="light"
										disabled={runtimeJobs.current.page <= 1}
										on:click={() =>
											(runtimeJobsCurrentPage = Math.max(1, runtimeJobsCurrentPage - 1))}
									>
										Previous
									</Button>
									<Button
										size="xs"
										color="light"
										disabled={runtimeJobs.current.page * runtimeJobs.current.per_page >=
											runtimeJobs.current.total}
										on:click={() => (runtimeJobsCurrentPage = runtimeJobsCurrentPage + 1)}
									>
										Next
									</Button>
								</div>
							</div>
						{/if}
					{/if}
				</section>
			{/if}
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
