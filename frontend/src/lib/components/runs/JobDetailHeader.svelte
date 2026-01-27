<script lang="ts">
	import { type Job } from '$lib/gen'
	import { base } from '$lib/base'
	import { displayDate, truncateRev } from '$lib/utils'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { workspaceStore } from '$lib/stores'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import {
		Clock,
		MemoryStick,
		Calendar,
		Bot,
		User,
		Code,
		IdCard,
		ExternalLink
	} from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import JobStatus from '$lib/components/JobStatus.svelte'
	import RunBadges from '$lib/components/runs/RunBadges.svelte'

	const SMALL_ICON_SIZE = 14

	interface Props {
		job: Job
		scheduleEditor?: ScheduleEditor
		displayPersistentScriptDefinition?: boolean
		openPersistentScriptDrawer?: () => void
		concurrencyKey?: string
		compact?: boolean
		extraCompact?: boolean
		onFilterByConcurrencyKey?: (key: string) => void
		onFilterByWorker?: (worker: string) => void
	}

	let {
		job,
		scheduleEditor,
		displayPersistentScriptDefinition = false,
		openPersistentScriptDrawer,
		concurrencyKey,
		compact = false,
		extraCompact = false,
		onFilterByConcurrencyKey,
		onFilterByWorker
	}: Props = $props()
</script>

{#if extraCompact}
	<!-- Extra compact variant: only status, ID and duration -->
	<div class="rounded-md border bg-surface-tertiary overflow-hidden w-full">
		<div class="flex flex-row flex-wrap justify-between items-center gap-x-4 py-2 px-3">
			<div class="flex flex-row flex-wrap gap-2 items-center">
				{#if job}
					<JobStatus {job} />
					<div class="flex items-center gap-1">
						<IdCard size={12} class="text-secondary flex-shrink-0" />
						<a
							href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}
							class="text-accent text-xs flex items-center gap-1"
						>
							<span class="truncate">{job.id}</span>
							<ExternalLink size={10} class="flex-shrink-0" />
						</a>
					</div>
				{/if}
			</div>
		</div>
	</div>
{:else}
	<div class="rounded-md border bg-surface-tertiary overflow-hidden w-full">
		<!-- Top section: Status, Path, Badges -->
		<div
			class="flex flex-row flex-wrap justify-between items-center gap-x-4 {compact
				? 'py-3 px-4'
				: 'py-4 px-6'}"
		>
			<div
				class="flex flex-wrap gap-y-2 {compact
					? 'gap-3 flex-col items-start'
					: 'flex-row gap-6 items-center'}"
			>
				{#if job}
					<div class="flex flex-row gap-4 items-center w-full">
						<JobStatus {job} />
						<span class="text-emphasis {compact ? 'text-sm' : 'text-lg'} font-semibold">
							{#if job.script_path}
								{job.script_path}
							{:else if job.job_kind == 'dependencies'}
								lock dependencies
							{:else if job.job_kind == 'flowdependencies'}
								flow dependencies
							{:else if job.job_kind == 'appdependencies'}
								app dependencies
							{:else if job.job_kind == 'deploymentcallback'}
								deployment callback
							{:else if job.job_kind == 'identity'}
								Identity job
							{:else if job.job_kind == 'script_hub'}
								Script from hub
							{:else if job.job_kind == 'aiagent'}
								AI Agent
							{:else}
								{job.job_kind || 'Unknown job type'}
							{/if}
						</span>
						{#if compact}
							<!-- Run ID for compact mode -->

							<a
								href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}
								class="text-accent text-xs flex items-center gap-1"
							>
								<span class="truncate" title={job.id}>{truncateRev(job.id, 10)}</span>
								<ExternalLink size={10} class="flex-shrink-0" />
							</a>
						{/if}
					</div>
					<div class="flex flex-row gap-2 items-center flex-wrap">
						<RunBadges
							{job}
							{displayPersistentScriptDefinition}
							{openPersistentScriptDrawer}
							{concurrencyKey}
							{onFilterByConcurrencyKey}
							{onFilterByWorker}
						/>
					</div>
				{/if}
			</div>
		</div>

		<!-- Bottom section: Metadata Grid -->
		{#if !compact}
			<div class="px-6 py-4">
				<div class="grid grid-cols-3 grid-rows-2 gap-x-8 gap-y-3 text-xs text-primary font-normal">
					{#if job}
						<!-- Row 1, Column 1: Received at -->
						<div class="flex flex-row gap-2 items-center">
							<Clock size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary" />
							<span class="whitespace-nowrap">
								{#if job['success'] != undefined}
									Received: {displayDate(job.created_at ?? '')}
								{:else}
									Received <TimeAgo date={job.created_at ?? ''} />
								{/if}
								<Tooltip small>{job?.created_at}</Tooltip>
							</span>
						</div>

						<!-- Row 1, Column 2: By -->
						<div class="flex items-center gap-2">
							<User size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary" />
							<span>
								By {truncateRev(job.created_by ?? 'unknown', 30)}
								{#if (job?.created_by?.length ?? 0) > 30}
									<Tooltip small>
										<div class="break-all">{job.created_by}</div>
									</Tooltip>
								{/if}
								{#if job.permissioned_as !== `u/${job.created_by}` && job.permissioned_as != job.created_by}
									<span class="text-secondary text-2xs"
										>but permissioned as {job.permissioned_as}</span
									>
								{/if}
							</span>
						</div>

						<!-- Row 1, Column 3: Memory -->
						{#if job && job['mem_peak']}
							<div class="flex flex-row gap-2 items-center">
								<MemoryStick size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
								<span>Memory peak: {(job['mem_peak'] / 1024).toPrecision(5)}MB</span>
							</div>
						{:else}
							<div class="flex flex-row gap-2 items-center opacity-50">
								<MemoryStick size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
								<span>No memory info</span>
							</div>
						{/if}

						<!-- Row 2, Column 1: Started at -->
						{#if job && 'started_at' in job && job.started_at}
							<div class="flex flex-row gap-2 items-center">
								<Clock size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary" />
								<span class="whitespace-nowrap">
									Started <TimeAgo agoOnlyIfRecent date={job.started_at ?? ''} />
									<Tooltip small>{job?.started_at}</Tooltip>
								</span>
							</div>
						{:else}
							<div class="flex flex-row gap-2 items-center opacity-50">
								<Clock size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary" />
								<span>Not started</span>
							</div>
						{/if}

						<!-- Row 2, Column 2: Run ID -->
						<div class="flex items-center gap-2 min-w-0">
							<IdCard size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary flex-shrink-0" />
							<div class="flex text-primary text-2xs min-w-0 flex-1 whitespace-nowrap">
								<span class="whitespace-nowrap">Run ID:&nbsp;</span>
								<a
									href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}
									class="text-accent flex items-center gap-1 min-w-0 flex-1"
								>
									<div class="truncate flex-shrink min-w-0">{job.id}</div>
									<ExternalLink size={12} class="flex-shrink-0" />
								</a>
							</div>
						</div>

						<!-- Row 2, Column 3 -->
						{#if job && job.parent_job}
							{#if job.is_flow_step}
								<div class="flex flex-row gap-2 items-center">
									<BarsStaggered size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary" />
									<span class="whitespace-nowrap">
										Step of flow
										<a
											href={`${base}/run/${job.parent_job}?workspace=${$workspaceStore}`}
											class="text-accent"
										>
											{truncateRev(job.parent_job, 8)}
											<ExternalLink size={12} class="inline-block" />
										</a>
									</span>
								</div>
							{:else}
								<div class="flex flex-row gap-2 items-center">
									<Bot size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary" />
									<span class="whitespace-nowrap">
										Triggered by
										<a
											href={`${base}/run/${job.parent_job}?workspace=${$workspaceStore}`}
											class="text-accent"
										>
											{truncateRev(job.parent_job, 8)}
										</a>
										<ExternalLink size={12} class="inline-block" />
									</span>
								</div>
							{/if}
						{:else if job && job.schedule_path}
							<div class="flex flex-row gap-2 w-full items-center">
								<Calendar size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary" />
								<span class="whitespace-nowrap">
									Schedule:
									<button
										class="text-accent"
										onclick={() =>
											scheduleEditor?.openEdit?.(job.schedule_path ?? '', job.job_kind == 'flow')}
									>
										{truncateRev(job.schedule_path, 20)}
										<ExternalLink size={12} class="inline-block" />
									</button>
								</span>
							</div>
						{:else if job && (job.job_kind == 'flow' || job.job_kind == 'script' || job.job_kind == 'singlestepflow')}
							{@const stem = job.job_kind === 'script' ? 'scripts' : 'flows'}
							{@const isScript = job.job_kind === 'script'}
							{@const viewHref = `${base}/${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
							<div class="flex flex-row gap-2 items-center min-w-0">
								{#if isScript}
									<Code size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary flex-shrink-0" />
								{:else}
									<BarsStaggered
										size={SMALL_ICON_SIZE}
										class="min-w-3.5 text-secondary flex-shrink-0"
									/>
								{/if}
								<span class="min-w-0 flex-1">
									<a
										href={viewHref}
										class="text-accent hover:underline flex items-center gap-1 min-w-0"
									>
										<span class="truncate flex-shrink min-w-0">
											{isScript
												? truncateRev(job?.script_hash?.toString() ?? '', 12)
												: (job?.script_path ?? '')}
										</span>
										<ExternalLink size={12} class="flex-shrink-0" />
									</a>
								</span>
							</div>
						{:else}
							<div class="flex flex-row gap-2 items-center opacity-50">
								<Code size={SMALL_ICON_SIZE} class="min-w-3.5 text-secondary" />
								<span>No path info</span>
							</div>
						{/if}
					{/if}
				</div>
			</div>
		{/if}
	</div>
{/if}
