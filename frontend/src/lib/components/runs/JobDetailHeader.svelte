<script lang="ts">
	import { type Job } from '$lib/gen'
	import { base } from '$lib/base'
	import { displayDate, truncateRev, truncateHash } from '$lib/utils'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { workspaceStore } from '$lib/stores'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { IdCard, ExternalLink, ListFilter, ChevronDown, Share2, Link, Copy } from 'lucide-svelte'
	import JobStatus from '$lib/components/JobStatus.svelte'
	import JobStatusIcon from '$lib/components/runs/JobStatusIcon.svelte'
	import RunBadges from '$lib/components/runs/RunBadges.svelte'
	import WorkerHostname from '$lib/components/WorkerHostname.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { getRelevantFields, getTriggerInfo, type FieldConfig } from './JobDetailFieldConfig'

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
		showScriptHashInBadges?: boolean
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
		onFilterByWorker,
		showScriptHashInBadges = false
	}: Props = $props()

	// Get adaptive field configuration based on job type
	const relevantFields = $derived(() => getRelevantFields(job))
	const triggerInfo = $derived(() => getTriggerInfo(job))

	// Expansion state for compact modes
	let isExpanded = $state(false)

	// Container width tracking for responsive layout
	let clientWidth = $state(0)
	let useOneColumn = $derived(clientWidth < 800)

	/**
	 * Renders the value for a field configuration
	 */
	function renderFieldValue(config: FieldConfig, job: Job): string {
		switch (config.field) {
			case 'created_at':
				return 'Received'
			case 'started_at':
				return 'Started'
			case 'trigger_info':
				return triggerInfo()?.type || 'Trigger'
			case 'run_id':
				return 'Run ID'
			default:
				return config.label
		}
	}

	/**
	 * Gets the display value for a field
	 */
	function getDisplayValue(config: FieldConfig, job: Job): string | null {
		const value = config.getValue(job)
		if (!value) return null

		switch (config.field) {
			case 'script_hash':
				return truncateHash(value.toString())
			case 'worker':
				return truncateRev(value, compact ? 15 : 20)
			case 'created_by':
				return truncateRev(value, 30)
			case 'parent_job':
				return truncateRev(value, 8)
			case 'schedule_path':
				return truncateRev(value, 20)
			case 'run_id':
				return value
			default:
				return value
		}
	}

	/**
	 * Checks if a field should show time ago format
	 */
	function shouldShowTimeAgo(config: FieldConfig, job: Job): boolean {
		return config.field === 'created_at' && job['success'] == undefined
	}
</script>

{#snippet fieldValueRenderer(config, job, value, href)}
	{#if config.field === 'created_at'}
		<Tooltip small>
			{#snippet text()}{job?.created_at}{/snippet}
			<span class="whitespace-nowrap">
				{#if shouldShowTimeAgo(config, job)}
					<TimeAgo date={job.created_at ?? ''} />
				{:else}
					{displayDate(job.created_at ?? '')}
				{/if}
			</span>
		</Tooltip>
	{:else if config.field === 'started_at' && 'started_at' in job}
		<Tooltip small>
			{#snippet text()}{job?.started_at}{/snippet}
			<span class="whitespace-nowrap">
				<TimeAgo agoOnlyIfRecent date={job.started_at ?? ''} />
			</span>
		</Tooltip>
	{:else if config.field === 'created_by'}
		<span>
			{#if job.permissioned_as !== `u/${job.created_by}` && job.permissioned_as != job.created_by}
				<Tooltip small>
					{#snippet text()}
						{#if (job?.created_by?.length ?? 0) > 30}
							Created by: {job.created_by}<br />
						{/if}
						But permissioned as {job.permissioned_as}
					{/snippet}
					{value}
					<span class="text-secondary"> ({job.permissioned_as})</span>
				</Tooltip>
			{:else if (job?.created_by?.length ?? 0) > 30}
				<Tooltip small>
					{#snippet text()}{job.created_by}{/snippet}
					{value}
					<span class="inline"><!-- empty wrapper for tooltip alignment --></span>
				</Tooltip>
			{/if}
		</span>
	{:else if config.field === 'worker'}
		<span title={value}>
			{#if onFilterByWorker}
				<Tooltip>
					{#snippet text()}
						This job was run on worker:
						<Button
							class="inline-text"
							size="xs2"
							color="light"
							onclick={() => job?.worker && onFilterByWorker?.(job.worker)}
						>
							{job?.worker}
							<ListFilter class="inline-block" size={10} />
						</Button>
						<br />
						<WorkerHostname worker={job.worker!} minTs={job?.['created_at']} />
					{/snippet}
					<button
						onclick={() => job?.worker && onFilterByWorker?.(job.worker)}
						class="flex items-center gap-1"
					>
						{value}
						<ExternalLink size={12} class="flex-shrink-0" />
					</button>
				</Tooltip>
			{:else}
				<Tooltip>
					{#snippet text()}
						This job was run on worker:
						<a href={`${base}/runs/?job_kinds=all&worker=${job?.worker}`}>
							{job?.worker}
						</a>
						<br />
						<WorkerHostname worker={job.worker!} minTs={job?.['created_at']} />
					{/snippet}
					<a
						href={`${base}/runs/?job_kinds=all&worker=${job?.worker}`}
						class="flex items-center gap-1 text-primary"
					>
						{value}
						<ExternalLink size={12} class="flex-shrink-0" />
					</a>
				</Tooltip>
			{/if}
		</span>
	{:else if config.field === 'schedule_path' && job.schedule_path}
		<span class="whitespace-nowrap" title={value}>
			<button
				onclick={() => scheduleEditor?.openEdit?.(job.schedule_path ?? '', job.job_kind == 'flow')}
				class="flex items-center gap-1"
			>
				{value}
				<ExternalLink size={12} class="flex-shrink-0" />
			</button>
		</span>
	{:else if config.field === 'parent_job' && job.parent_job}
		<span class="whitespace-nowrap" title={value}>
			{#if job.is_flow_step}
				Step of flow
			{:else}
				Triggered by
			{/if}
			<a
				href={`${base}/run/${job.parent_job}?workspace=${$workspaceStore}`}
				class="flex items-center gap-1"
			>
				{value}
				<ExternalLink size={12} class="flex-shrink-0" />
			</a>
		</span>
	{:else if config.field === 'run_id'}
		<div class="flex items-center gap-2 whitespace-nowrap">
			<span title={value}>{truncateRev(value, 15)}</span>
			<DropdownV2
				size="xs"
				items={[
					{
						displayName: 'Copy URL',
						icon: Link,
						action: () =>
							navigator.clipboard.writeText(
								`${window.location.origin}${base}/run/${job.id}?workspace=${job.workspace_id}`
							)
					},
					{
						displayName: 'Copy Job ID',
						icon: Copy,
						action: () => navigator.clipboard.writeText(job.id)
					}
				]}
				class="-my-2"
			>
				{#snippet buttonReplacement()}
					<Button
						variant="subtle"
						unifiedSize="xs"
						startIcon={{ icon: Share2 }}
						iconOnly
						btnClasses="bg-transparent"
					/>
				{/snippet}
			</DropdownV2>
		</div>
	{:else if config.field === 'trigger_info'}
		{@const triggerInfoText = value + (triggerInfo()?.detail ? `: ${triggerInfo()?.detail}` : '')}
		<span title={triggerInfoText}>{triggerInfoText}</span>
	{:else if href}
		<a
			{href}
			class="flex items-center gap-1 min-w-0 text-primary"
			title={config.field === 'script_hash' ? `Script hash: ${job.script_hash}` : undefined}
		>
			<span class="truncate flex-shrink min-w-0">{value}</span>
			<ExternalLink size={12} class="flex-shrink-0" />
		</a>
	{:else}
		<span title={value} class="truncate">{value}</span>
	{/if}
{/snippet}

{#if extraCompact}
	<!-- Extra compact variant: only status, ID and expandable chevron -->
	<div class="rounded-md border bg-surface-tertiary overflow-hidden w-full">
		<div class="flex flex-row flex-wrap justify-between items-center gap-x-4 py-2 px-3">
			<div class="flex flex-row flex-wrap gap-2 items-center flex-1">
				{#if job}
					<JobStatus {job} />
					<div class="flex items-center gap-1">
						<IdCard size={12} class="text-secondary flex-shrink-0" />
						<a
							href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}
							class="text-xs flex items-center gap-1"
						>
							<span class="truncate">{job.id}</span>
							<ExternalLink size={10} class="flex-shrink-0" />
						</a>
					</div>
				{/if}
			</div>
			<!-- Expansion toggle -->
			{#if relevantFields().filter((f) => f.field !== 'run_id' && f.field !== 'created_at').length > 0}
				<button
					onclick={() => (isExpanded = !isExpanded)}
					class="text-secondary hover:text-primary transition-colors p-1 rounded"
					title={isExpanded ? 'Show less' : 'Show more details'}
				>
					<ChevronDown size={14} class={isExpanded ? 'transform rotate-180' : ''} />
				</button>
			{/if}
		</div>

		<!-- Expanded content for extra compact -->
		{#if isExpanded}
			{@const expandedFields = relevantFields()
				.filter((f) => f.field !== 'run_id' && f.field !== 'created_at')
				.slice(0, 3)}
			<div class="px-3 pb-2 border-t border-surface-secondary bg-surface">
				<div class="flex flex-wrap gap-x-4 gap-y-1 text-2xs text-secondary pt-2">
					{#each expandedFields as config}
						{@const value = getDisplayValue(config, job)}
						{@const IconComponent =
							config.field === 'trigger_info' && triggerInfo() ? triggerInfo()?.icon : config.icon}
						{#if value}
							<div class="flex items-center gap-1">
								<IconComponent size={10} class="text-tertiary flex-shrink-0" />
								<span class="truncate"
									>{config.label}: {@render fieldValueRenderer(
										config,
										job,
										value,
										config.getHref?.(job, $workspaceStore || '')
									)}</span
								>
							</div>
						{/if}
					{/each}
				</div>
			</div>
		{/if}
	</div>
{:else}
	<div class="rounded-md border bg-surface-tertiary overflow-hidden w-full" bind:clientWidth>
		<!-- Top section: Title with Status Dot and Badges Below -->
		<div class={compact ? 'py-3 px-4' : 'py-6 px-8'}>
			{#if job}
				<!-- Header with status icon and two-row title/badges section -->
				<div class="flex items-center gap-3">
					<!-- Status icon -->
					<div class="flex-shrink-0">
						<JobStatusIcon {job} roundedFull />
					</div>

					<!-- Two-row section: title on top, badges on bottom -->
					<div class="flex flex-col gap-.5 flex-1 min-w-0">
						<!-- Title row -->
						<div>
							{#if job.script_path && (job.job_kind === 'script' || job.job_kind === 'flow' || job.job_kind === 'singlestepflow')}
								{@const stem = job.job_kind === 'script' ? 'scripts' : 'flows'}
								{@const isScript = job.job_kind === 'script'}
								{@const viewHref = `${base}/${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
								<a
									href={viewHref}
									class="text-emphasis {compact
										? 'text-base'
										: 'text-lg'} font-semibold flex items-center gap-1"
								>
									{job.script_path}
									<ExternalLink size={14} class="flex-shrink-0" />
								</a>
							{:else}
								<span class="text-emphasis {compact ? 'text-base' : 'text-lg'} font-semibold">
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
							{/if}
						</div>

						<!-- Badges row -->

						<!-- Job Status -->
						<div class="flex items-baseline flex-wrap gap-2">
							<JobStatus {job} />

							<RunBadges
								{job}
								{displayPersistentScriptDefinition}
								{openPersistentScriptDrawer}
								{concurrencyKey}
								{onFilterByConcurrencyKey}
								showScriptHash={showScriptHashInBadges}
								large={false}
							/>
						</div>
					</div>
				</div>
			{/if}
		</div>

		<!-- Separation bar -->
		<div class="border-t mx-8"></div>

		<!-- Bottom section: Adaptive Metadata in single grid layout -->
		{#if !compact}
			{@const fields = relevantFields()}
			<div class="px-8 py-6">
				<div
					class="grid gap-x-12 gap-y-2"
					class:grid-cols-1={useOneColumn}
					class:grid-cols-2={!useOneColumn}
				>
					{#if job}
						{#each fields as config}
							{@const value = getDisplayValue(config, job)}
							{@const href = config.getHref?.(job, $workspaceStore || '')}

							{#if value}
								<div class="flex items-baseline gap-3 text-xs">
									<span class="text-secondary min-w-[70px] flex-shrink-0">
										{config.label}
									</span>
									<span class="text-primary min-w-0 flex-1">
										<div class="truncate">
											{@render fieldValueRenderer(config, job, value, href)}
										</div>
									</span>
								</div>
							{/if}
						{/each}
					{/if}
				</div>
			</div>
		{:else}
			<!-- Compact version: Job ID prominently displayed + adaptive fields + expansion -->
			{@const fields = relevantFields()
				.filter((f) => f.field !== 'run_id')
				.slice(0, 2)}
			<!-- Exclude run_id since we show it separately, limit to 2 other fields -->
			{@const additionalFieldsCount = relevantFields().length - fields.length - 1}
			<!-- -1 for run_id -->
			<div class="px-4 py-2">
				<div
					class="flex flex-wrap justify-between items-start gap-x-4 gap-y-2 text-xs text-primary font-normal"
				>
					<div class="flex flex-wrap gap-y-2 flex-1 items-center">
						{#if job}
							<!-- Always show Job ID prominently in compact mode -->
							<div class="flex items-baseline gap-1 text-xs">
								<span class="text-secondary flex-shrink-0">Job ID</span>
								<span class="text-primary">
									<a
										href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}
										class="flex items-center gap-1 min-w-0"
									>
										<span class="truncate flex-shrink min-w-0">{job.id}</span>
										<ExternalLink size={12} class="flex-shrink-0" />
									</a>
								</span>
							</div>

							{#each fields as config (config.field)}
								{@const value = getDisplayValue(config, job)}
								{@const href = config.getHref?.(job, $workspaceStore || '')}

								{#if value}
									<!-- Separator -->
									<div class="flex items-center px-3">
										<div class="w-px h-4 bg-border-light"></div>
									</div>

									<!-- Field -->
									<div class="flex items-baseline gap-1 text-xs min-w-0">
										<span class="text-secondary flex-shrink-0">{config.label}</span>
										<span class="text-primary min-w-0 flex-1">
											<div class="truncate" title={value}>
												{@render fieldValueRenderer(config, job, value, href)}
											</div>
										</span>
									</div>
								{/if}
							{/each}
						{/if}
					</div>

					<!-- Expansion button for compact mode -->
					{#if additionalFieldsCount > 0}
						<Button
							variant="subtle"
							unifiedSize="sm"
							onclick={() => (isExpanded = !isExpanded)}
							title="More details"
						>
							More details
							<ChevronDown
								size={14}
								class={isExpanded ? 'transform rotate-180 transition-transform' : ''}
							/>
						</Button>
					{/if}
				</div>

				<!-- Expanded content for compact mode -->
				{#if isExpanded && additionalFieldsCount > 0}
					{@const expandedFields = relevantFields()
						.filter((f) => f.field !== 'run_id')
						.slice(2)}
					<!-- Show remaining fields in single column -->
					<div class="mt-2 pt-2 border-t border-surface-secondary">
						<div class="flex flex-col gap-y-3">
							{#each expandedFields as config}
								{@const value = getDisplayValue(config, job)}
								{@const href = config.getHref?.(job, $workspaceStore || '')}

								{#if value}
									<div class="flex items-baseline gap-3 text-xs">
										<span class="text-secondary min-w-[70px] flex-shrink-0">
											{#if config.field === 'created_at'}
												{renderFieldValue(config, job)}
											{:else}
												{config.label}
											{/if}
										</span>
										<span class="text-primary min-w-0 flex-1">
											<div class="truncate" title={value}>
												{@render fieldValueRenderer(config, job, value, href)}
											</div>
										</span>
									</div>
								{/if}
							{/each}
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}
