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

	interface AssetJobInfo {
		id: string
		created_at: string
		created_by: string
		runnable_path?: string
		status?: 'success' | 'failure' | 'canceled' | 'skipped'
	}

	interface AssetJobListResponse {
		jobs: AssetJobInfo[]
		total: number
		page: number
		per_page: number
	}

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

	let runtimeJobs: AssetJobListResponse | undefined = $state()
	let loadingJobs = $state(false)
	let currentPage = $state(1)

	export function open(data: typeof usagesDrawerData) {
		usagesDrawerData = data
		currentPage = 1
		runtimeJobs = undefined
		if (data && data.usages.some((u) => u.detection_kinds?.includes('runtime'))) {
			loadRuntimeJobs(1)
		}
	}

	async function loadRuntimeJobs(page: number) {
		if (!usagesDrawerData || !$workspaceStore) return

		loadingJobs = true
		try {
			const response = await AssetService.listAssetJobs({
				workspace: $workspaceStore,
				assetPath: usagesDrawerData.path,
				assetKind: usagesDrawerData.kind,
				page,
				perPage: 20
			})
			runtimeJobs = response as AssetJobListResponse
			currentPage = page
		} catch (error) {
			console.error('Failed to load runtime jobs:', error)
		} finally {
			loadingJobs = false
		}
	}

	function getStatusColor(status?: string): string {
		switch (status) {
			case 'success':
				return 'text-green-600'
			case 'failure':
				return 'text-red-600'
			case 'canceled':
				return 'text-yellow-600'
			case 'skipped':
				return 'text-gray-600'
			default:
				return 'text-blue-600'
		}
	}

	function getStatusLabel(status?: string): string {
		return status ? status.charAt(0).toUpperCase() + status.slice(1) : 'Running'
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
					<h3 class="text-sm font-semibold mb-2">Recent Job Executions</h3>

					{#if loadingJobs}
						<div class="flex items-center justify-center py-8 text-sm text-secondary">
							Loading jobs...
						</div>
					{:else if runtimeJobs}
						<div class="flex flex-col border rounded-md divide-y">
							{#each runtimeJobs.jobs as job}
								<a
									href={`/run/${job.id}?workspace=${$workspaceStore}`}
									class="text-xs text-primary font-normal flex items-center py-3 px-4 gap-3 hover:bg-surface-hover cursor-pointer"
								>
									<div class="flex flex-col justify-center flex-1">
										<div class="flex items-center gap-2">
											<span class="font-semibold text-emphasis">
												{job.runnable_path || 'Inline script'}
											</span>
											<span class={twMerge('text-2xs font-medium', getStatusColor(job.status))}>
												{getStatusLabel(job.status)}
											</span>
										</div>
										<div class="text-2xs text-secondary flex items-center gap-2">
											<span>{job.created_by}</span>
											<span>•</span>
											<span>{displayDate(job.created_at)}</span>
										</div>
									</div>
									{@render badge(job.id.substring(0, 8), 'Job ID')}
								</a>
							{/each}
						</div>

						<!-- Pagination -->
						{#if runtimeJobs.total > runtimeJobs.per_page}
							<div class="flex items-center justify-between mt-3 text-sm">
								<span class="text-secondary">
									Showing {(runtimeJobs.page - 1) * runtimeJobs.per_page + 1} -
									{Math.min(runtimeJobs.page * runtimeJobs.per_page, runtimeJobs.total)} of {runtimeJobs.total}
									jobs
								</span>
								<div class="flex gap-2">
									<Button
										size="xs"
										color="light"
										disabled={runtimeJobs.page <= 1}
										on:click={() => loadRuntimeJobs(currentPage - 1)}
									>
										Previous
									</Button>
									<Button
										size="xs"
										color="light"
										disabled={runtimeJobs.page * runtimeJobs.per_page >= runtimeJobs.total}
										on:click={() => loadRuntimeJobs(currentPage + 1)}
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
