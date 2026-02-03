<script lang="ts">
	import { type Job } from '$lib/gen'
	import { base } from '$lib/base'
	import { displayDate, truncateRev, truncateHash } from '$lib/utils'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { workspaceStore } from '$lib/stores'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { ExternalLink, ListFilter, ChevronDown, Share2, Link, Copy } from 'lucide-svelte'
	import JobStatus from '$lib/components/JobStatus.svelte'
	import JobStatusIcon from '$lib/components/runs/JobStatusIcon.svelte'
	import RunBadges from '$lib/components/runs/RunBadges.svelte'
	import WorkerHostname from '$lib/components/WorkerHostname.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { getRelevantFields, getTriggerInfo, type FieldConfig } from './JobDetailFieldConfig'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'

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

	// Grid now uses auto-fit for smart responsive layout

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
	 * Gets the raw value for a field (without truncation)
	 */
	function getFullValue(config: FieldConfig, job: Job): string {
		const value = config.getValue(job)
		if (!value) return 'no value'
		return value
	}

	/**
	 * Gets the truncated display value for a field
	 */
	function getTruncatedValue(config: FieldConfig, job: Job, compact: boolean = false): string {
		const fullValue = getFullValue(config, job)
		if (fullValue === 'no value') return fullValue

		switch (config.field) {
			case 'run_id':
				return truncateRev(fullValue, 8)
			case 'script_hash':
				return truncateHash(fullValue.toString())
			case 'worker':
				return truncateRev(fullValue, compact ? 8 : 12)
			case 'parent_job':
				return truncateRev(fullValue, 6)
			case 'schedule_path':
				return truncateRev(fullValue, 20)
			default:
				return fullValue
		}
	}

	/**
	 * Gets the display value for a field
	 */
	function getDisplayValue(config: FieldConfig, job: Job): string {
		return getTruncatedValue(config, job, compact)
	}

	/**
	 * Checks if a field should show time ago format
	 */
	function shouldShowTimeAgo(config: FieldConfig, job: Job): boolean {
		return config.field === 'created_at' && job['success'] == undefined
	}

	function getJobKindDisplayName(job: Job): string {
		return job.script_path
			? job.script_path
			: job.job_kind === 'dependencies'
				? 'lock dependencies'
				: job.job_kind === 'flowdependencies'
					? 'flow dependencies'
					: job.job_kind === 'appdependencies'
						? 'app dependencies'
						: job.job_kind === 'deploymentcallback'
							? 'deployment callback'
							: job.job_kind === 'identity'
								? 'Identity job'
								: job.job_kind === 'script_hub'
									? 'Script from hub'
									: job.job_kind === 'aiagent'
										? 'AI Agent'
										: job.job_kind || 'Unknown job type'
	}
</script>

{#snippet fieldValueRenderer(config, job, displayValue, fullValue, href)}
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
		<div class="flex items-center gap-1 min-w-0">
			{#if job.permissioned_as !== `u/${job.created_by}` && job.permissioned_as != job.created_by}
				<Tooltip small class="truncate" style="direction: rtl;">
					{#snippet text()}
						{#if (job?.created_by?.length ?? 0) > 30}
							Created by: {job.created_by}<br />
						{/if}
						But permissioned as {job.permissioned_as}
					{/snippet}

					<span
						class="text-secondary flex-shrink whitespace-nowrap overflow-hidden"
						style="direction: rtl;"
					>
						<span class="text-primary">{displayValue}</span> ({job.permissioned_as})
					</span>
				</Tooltip>
			{:else}
				<Tooltip small>
					{#snippet text()}{job.created_by}{/snippet}
					<span>{displayValue}</span>
				</Tooltip>
			{/if}
		</div>
	{:else if config.field === 'worker'}
		<span>
			{#if displayValue === 'no value'}
				{displayValue}
			{:else if onFilterByWorker}
				<Tooltip>
					{#snippet text()}
						This job was run on worker:
						<Button
							variant="subtle"
							unifiedSize="xs"
							endIcon={{ icon: ListFilter }}
							onClick={() => job?.worker && onFilterByWorker?.(job.worker)}
							wrapperClasses="w-fit"
						>
							{job?.worker}
						</Button>
						<br />
						<WorkerHostname worker={job.worker!} minTs={job?.['created_at']} />
					{/snippet}
					<button
						onclick={() => job?.worker && onFilterByWorker?.(job.worker)}
						class="flex items-center gap-1 font-normal"
					>
						{displayValue}
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
						{displayValue}
						<ExternalLink size={12} class="flex-shrink-0" />
					</a>
				</Tooltip>
			{/if}
		</span>
	{:else if config.field === 'schedule_path' && job.schedule_path}
		<span class="whitespace-nowrap" title={fullValue}>
			<button
				onclick={() => scheduleEditor?.openEdit?.(job.schedule_path ?? '', job.job_kind == 'flow')}
				class="flex items-center gap-1"
			>
				{displayValue}
				<ExternalLink size={12} class="flex-shrink-0" />
			</button>
		</span>
	{:else if config.field === 'parent_job' && job.parent_job}
		<span class="whitespace-nowrap flex items-center gap-1" title={fullValue}>
			{#if job.is_flow_step}
				Step of flow
			{:else}
				Triggered by
			{/if}
			<a
				href={`${base}/run/${job.parent_job}?workspace=${$workspaceStore}`}
				class="flex items-center gap-1"
			>
				{displayValue}
				<ExternalLink size={12} class="flex-shrink-0" />
			</a>
		</span>
	{:else if config.field === 'run_id'}
		<div class="flex items-center gap-2 whitespace-nowrap">
			<span title={fullValue}>{displayValue}</span>
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
		{@const triggerInfoText =
			displayValue + (triggerInfo()?.detail ? `: ${triggerInfo()?.detail}` : '')}
		<span title={triggerInfoText}>{triggerInfoText}</span>
	{:else if href}
		<a
			{href}
			class="flex items-center gap-1 min-w-0 text-primary"
			title={config.field === 'script_hash' ? `Script hash: ${fullValue}` : fullValue}
		>
			<span class="truncate flex-shrink min-w-0">{displayValue}</span>
			<ExternalLink size={12} class="flex-shrink-0" />
		</a>
	{:else}
		<span title={fullValue} class="truncate">{displayValue}</span>
	{/if}
{/snippet}

{#if extraCompact}
	<!-- Extra compact variant: only status, ID and expandable chevron -->
	<div class="rounded-md border bg-surface-tertiary overflow-auto w-full">
		<div class="flex flex-row flex-wrap justify-between items-center gap-x-4 py-2 px-3">
			<div class="flex flex-row flex-wrap gap-2 items-center flex-1">
				{#if job}
					<JobStatus {job} />
					<div class="flex items-baseline gap-1 text-xs">
						<span class="text-secondary flex-shrink-0">Job ID</span>
						<span class="text-primary">
							<a
								href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}
								class="flex items-center gap-1 min-w-0"
							>
								<span class="truncate flex-shrink min-w-0">{truncateRev(job.id, 8)}</span>
								<ExternalLink size={10} class="flex-shrink-0" />
							</a>
						</span>
					</div>
				{/if}
			</div>
			<!-- Expansion toggle -->
			{#if relevantFields().filter((f) => f.field !== 'run_id' && f.field !== 'created_at').length > 0}
				<Button
					variant="subtle"
					unifiedSize="sm"
					onClick={() => (isExpanded = !isExpanded)}
					title={isExpanded ? 'Show less' : 'Show more details'}
					iconOnly
					startIcon={{
						icon: ChevronDown,
						classes: isExpanded ? 'transform rotate-180 transition-transform' : ''
					}}
				/>
			{/if}
		</div>

		<!-- Expanded content for extra compact -->
		{#if isExpanded}
			{@const expandedFields = relevantFields()
				.filter((f) => f.field !== 'run_id' && f.field !== 'created_at')
				.slice(0, 3)}
			<div
				class="px-3 pb-2 border-t border-surface-secondary/50 bg-surface"
				transition:slide={{ duration: 150 }}
			>
				<div class="flex flex-col gap-y-1 text-xs pt-2">
					{#each expandedFields as config}
						{@const displayValue = getDisplayValue(config, job)}
						{@const fullValue = getFullValue(config, job)}
						{@const href = config.getHref?.(job, $workspaceStore || '')}

						<div class="flex items-baseline gap-1 text-xs">
							<span class="text-secondary flex-shrink-0">{config.label}</span>
							<span
								class:text-primary={displayValue !== 'no value'}
								class:text-secondary={displayValue === 'no value'}
								class="min-w-0 flex-1"
							>
								<div class:truncate={config.field !== 'created_by'}>
									{@render fieldValueRenderer(config, job, displayValue, fullValue, href)}
								</div>
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>
{:else}
	<div
		class={twMerge(
			'rounded-md border overflow-auto w-full flex',
			compact ? 'flex-col' : 'bg-border-light flex-wrap items-stretch gap-y-[1px] gap-x-[1px]'
		)}
	>
		<!-- Top section: Title with Status Dot and Badges Below -->
		<div
			class={twMerge(
				compact ? 'py-3 px-4 min-w-0 grow' : 'py-6 px-8 flex items-center min-w-0 grow',
				'bg-surface-tertiary'
			)}
			style={compact ? '' : 'flex: 1 1 min-content;'}
		>
			{#if job}
				<!-- Header with status icon and two-row title/badges section -->
				<div class="flex items-center gap-3 min-w-0 grow">
					<!-- Status icon -->
					<div class="flex-shrink-0">
						<JobStatusIcon {job} roundedFull />
					</div>

					<!-- Two-row section: title on top, badges on bottom -->
					<div class="flex flex-col gap-1 flex-1 min-w-0">
						<!-- Title row -->
						<div class="min-w-0 grow">
							{#if job.script_path && (job.job_kind === 'script' || job.job_kind === 'flow' || job.job_kind === 'singlestepflow')}
								{@const stem = job.job_kind === 'script' ? 'scripts' : 'flows'}
								{@const isScript = job.job_kind === 'script'}
								{@const viewHref = `${base}/${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
								<a
									href={viewHref}
									class="text-emphasis {compact
										? 'text-xs'
										: 'text-lg'} font-semibold flex items-center gap-1"
								>
									<span class="truncate" title={job.script_path}>{job.script_path}</span>
									<ExternalLink size={14} class="flex-shrink-0" />
								</a>
							{:else}
								{@const displayName = getJobKindDisplayName(job)}
								<div
									class="text-emphasis {compact ? 'text-xs' : 'text-lg'} font-semibold truncate"
									title={displayName}>{displayName}</div
								>
							{/if}
						</div>

						<!-- Badges row -->

						<!-- Job Status -->
						<div class="flex items-baseline flex-wrap gap-x-2 gap-y-1">
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

		<!-- Bottom section: Adaptive Metadata in single grid layout -->
		{#if !compact}
			{@const fields = relevantFields()}
			<div class="bg-surface flex items-center" style="flex: 2 1 100px; min-width: 350px;">
				<div
					class="grid gap-x-6 gap-y-1.5 w-full px-8 py-4 bg-surface-secondary/30"
					style="grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));"
				>
					{#if job}
						{#each fields as config}
							{@const displayValue = getDisplayValue(config, job)}
							{@const fullValue = getFullValue(config, job)}
							{@const href = config.getHref?.(job, $workspaceStore || '')}

							<div class="flex items-baseline gap-3 text-xs">
								<span class="text-secondary min-w-[70px] flex-shrink-0">
									{config.label}
								</span>
								<span
									class:text-primary={displayValue !== 'no value'}
									class:text-secondary={displayValue === 'no value'}
									class="min-w-0 flex-1"
								>
									<div class:truncate={config.field !== 'created_by'}>
										{@render fieldValueRenderer(config, job, displayValue, fullValue, href)}
									</div>
								</span>
							</div>
						{/each}
					{/if}
				</div>
			</div>
		{:else}
			<!-- Compact version: Job ID prominently displayed + adaptive fields + expansion -->
			{@const fields = relevantFields()
				.filter((f) => f.field !== 'run_id')
				.slice(0, 1)}
			<!-- Exclude run_id since we show it separately, limit to 2 other fields -->
			{@const additionalFieldsCount = relevantFields().length - fields.length - 1}
			<!-- -1 for run_id -->
			<div class="px-4 py-2 bg-surface-secondary/30 border-t">
				<div
					class="flex flex-wrap justify-between items-start gap-x-4 gap-y-1 text-xs text-primary font-normal"
				>
					<div class="flex flex-wrap gap-y-2 gap-x-6 flex-1 items-center">
						{#if job}
							<!-- Always show Job ID prominently in compact mode -->
							<div class="flex items-baseline gap-1 text-xs">
								<span class="text-secondary flex-shrink-0">Job ID</span>
								<span class="text-primary">
									<a
										href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}
										class="flex items-center gap-1 min-w-0"
									>
										<span class="truncate flex-shrink min-w-0">{truncateRev(job.id, 8)}</span>
										<ExternalLink size={12} class="flex-shrink-0" />
									</a>
								</span>
							</div>

							{#each fields as config (config.field)}
								{@const displayValue = getDisplayValue(config, job)}
								{@const fullValue = getFullValue(config, job)}
								{@const href = config.getHref?.(job, $workspaceStore || '')}

								<!-- Field -->
								<div class="flex items-baseline gap-1 text-xs min-w-0">
									<span class="text-secondary flex-shrink-0">{config.label}</span>
									<span
										class:text-primary={displayValue !== 'no value'}
										class:text-secondary={displayValue === 'no value'}
										class="min-w-0 flex-1"
									>
										<div class:truncate={config.field !== 'created_by'} title={fullValue}>
											{@render fieldValueRenderer(config, job, displayValue, fullValue, href)}
										</div>
									</span>
								</div>
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
							wrapperClasses="-my-1.5"
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
						.slice(1)}
					<!-- Show remaining fields in single column -->
					<div class="mt-2 pt-2 border-t" transition:slide={{ duration: 150 }}>
						<div class="flex flex-col gap-y-1">
							{#each expandedFields as config}
								{@const displayValue = getDisplayValue(config, job)}
								{@const fullValue = getFullValue(config, job)}
								{@const href = config.getHref?.(job, $workspaceStore || '')}

								<div class="flex items-baseline gap-3 text-xs">
									<span class="text-secondary min-w-[70px] flex-shrink-0">
										{#if config.field === 'created_at'}
											{renderFieldValue(config, job)}
										{:else}
											{config.label}
										{/if}
									</span>
									<span
										class:text-primary={displayValue !== 'no value'}
										class:text-secondary={displayValue === 'no value'}
										class="min-w-0 flex-1"
									>
										<div
											class:truncate={config.field !== 'created_by'}
											class:truncate-start={config.field === 'created_by'}
											title={fullValue}
										>
											{@render fieldValueRenderer(config, job, displayValue, fullValue, href)}
										</div>
									</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.truncate-start {
		overflow: hidden;
		white-space: nowrap;
		direction: rtl;
		text-align: left;
	}

	.truncate-start > * {
		direction: ltr;
	}
</style>
