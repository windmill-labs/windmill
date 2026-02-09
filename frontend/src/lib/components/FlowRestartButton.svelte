<script lang="ts">
	import { Badge, Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import { Play, RefreshCw } from 'lucide-svelte'
	import { FlowService, JobService, type FlowVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptyString, sendUserToast } from '$lib/utils'

	interface Props {
		jobId: string
		selectedJobStep: string
		selectedJobStepType: 'single' | 'forloop' | 'branchall'
		restartBranchNames?: [number, string][]
		flowPath?: string
		/** The flow version ID used in this run (from job.script_hash) */
		flowVersionId?: number
		disabled?: boolean
		enterpriseOnly?: boolean
		variant?: 'default' | 'accent'
		unifiedSize?: 'xs' | 'sm' | 'md' | 'lg'
		/** Called when flow is restarted. If not provided, will navigate to the new run using goto (requires SvelteKit) */
		onRestart?: (stepId: string, branchOrIterationN: number, flowVersion?: number) => void
		/** Called when flow restart completes with the new job ID. Used for navigation in non-SvelteKit contexts */
		onRestartComplete?: (newJobId: string) => void
	}

	let {
		jobId,
		selectedJobStep,
		selectedJobStepType,
		restartBranchNames = [],
		flowPath = undefined,
		flowVersionId = undefined,
		disabled = false,
		enterpriseOnly = false,
		variant = 'default',
		unifiedSize = 'md',
		onRestart,
		onRestartComplete
	}: Props = $props()

	// Sentinel value meaning "use the same version as the original run" (backend receives undefined)
	const RUN_VERSION_SENTINEL = -1

	let branchOrIterationN = $state(0)
	let selectedFlowVersion: number = $state(RUN_VERSION_SENTINEL)
	let flowVersions: Array<FlowVersion> = $state([])
	let loadingVersions: boolean = $state(false)
	let versionsLoaded: boolean = $state(false)
	let runVersionInList: boolean = $state(false)

	async function restartFlow(stepId: string, branchOrIterationN: number, flowVersion?: number) {
		let run = await JobService.restartFlowAtStep({
			workspace: $workspaceStore!,
			id: jobId,
			requestBody: {
				step_id: stepId,
				branch_or_iteration_n: branchOrIterationN,
				flow_version: flowVersion
			}
		})
		onRestartComplete?.(run)
	}

	async function loadFlowVersions() {
		if (!flowPath || loadingVersions || versionsLoaded) return
		loadingVersions = true
		try {
			flowVersions = await FlowService.getFlowHistory({
				workspace: $workspaceStore!,
				path: flowPath
			})
			if (flowVersions.length > 0) {
				const match = flowVersionId ? flowVersions.find((v) => v.id === flowVersionId) : undefined
				runVersionInList = match !== undefined
				selectedFlowVersion =
					match?.id ?? (flowVersionId ? RUN_VERSION_SENTINEL : flowVersions[0].id)
			}
			versionsLoaded = true
		} catch (e) {
			sendUserToast('Failed to load flow versions', true)
		} finally {
			loadingVersions = false
		}
	}

	function getFlowVersionForRestart(): number | undefined {
		return selectedFlowVersion === RUN_VERSION_SENTINEL ? undefined : selectedFlowVersion
	}

	function formatVersionLabel(version: FlowVersion): string {
		const name = emptyString(version.deployment_msg) ? `v${version.id}` : version.deployment_msg!
		return `${name} - ${new Date(version.created_at).toLocaleString()}`
	}

	function handleRestart() {
		if (onRestart) {
			onRestart(selectedJobStep, branchOrIterationN, getFlowVersionForRestart())
		} else {
			const flowVersion = getFlowVersionForRestart()
			restartFlow(selectedJobStep, branchOrIterationN, flowVersion)
		}
	}
</script>

{#snippet flowVersionSelector()}
	<label>
		<div class="pb-1 text-xs font-semibold text-emphasis">Flow version</div>
		{#if loadingVersions}
			<div class="text-xs text-secondary">Loading versions...</div>
		{:else if flowVersions.length > 0}
			<select bind:value={selectedFlowVersion} class="w-full text-xs">
				{#if flowVersionId && !runVersionInList}
					<option value={RUN_VERSION_SENTINEL}>Same as run (v{flowVersionId})</option>
				{/if}
				{#each flowVersions as version, i (version.id)}
					{@const isLatest = i === 0}
					{@const isSameAsRun = flowVersionId !== undefined && version.id === flowVersionId}
					<option value={version.id}>
						{formatVersionLabel(version)}{isSameAsRun
							? ' (Same as run)'
							: isLatest
								? ' (Latest)'
								: ''}
					</option>
				{/each}
			</select>
		{:else}
			<div class="text-xs text-tertiary">No versions available</div>
		{/if}
	</label>
{/snippet}
{#snippet singleRestartButton()}
	<Button
		title={`Re-start this flow from step ${selectedJobStep} (included).${enterpriseOnly ? ' This is a feature only available in enterprise edition.' : ''}`}
		{variant}
		{unifiedSize}
		{disabled}
		startIcon={{ icon: Play }}
		nonCaptureEvent={!!flowPath}
		onClick={() => {
			if (!flowPath) {
				handleRestart()
			}
		}}
	>
		Re-start from
		<Badge baseClass="ml-1" color="indigo">
			{selectedJobStep}
		</Badge>
		{#if enterpriseOnly && disabled}
			(EE)
		{/if}
	</Button>
{/snippet}

{#if selectedJobStepType === 'single'}
	{#if !flowPath}
		{@render singleRestartButton()}
	{:else}
		<Popover
			floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
			disablePopup={!flowPath}
			on:openChange={(e) => { if (e.detail) loadFlowVersions() }}
		>
			{#snippet trigger()}
				{@render singleRestartButton()}
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col gap-4 text-primary p-4 w-80">
					{@render flowVersionSelector()}

					<Button variant="accent" onClick={handleRestart}>Restart</Button>
				</div>
			{/snippet}
		</Popover>
	{/if}
{:else}
	<Popover
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
		on:openChange={(e) => { if (e.detail) loadFlowVersions() }}
	>
		{#snippet trigger()}
			<Button
				title={`Re-start this flow from step ${selectedJobStep} (included).${enterpriseOnly ? ' This is a feature only available in enterprise edition.' : ''}`}
				{variant}
				{unifiedSize}
				{disabled}
				startIcon={{ icon: RefreshCw }}
				nonCaptureEvent={true}
			>
				Re-start from
				<Badge baseClass="ml-1" color="indigo">
					{selectedJobStep}
				</Badge>
				{#if enterpriseOnly && disabled}
					(EE)
				{/if}
			</Button>
		{/snippet}
		{#snippet content()}
			<div class="flex flex-col gap-4 text-primary p-4 w-80">
				<label>
					<div class="pb-1 text-xs font-semibold text-emphasis"
						>{selectedJobStepType == 'forloop' ? 'From iteration #' : 'From branch'}</div
					>
					<div class="flex w-full gap-2">
						{#if selectedJobStepType === 'forloop'}
							<input type="number" min="0" bind:value={branchOrIterationN} class="!w-32 grow" />
						{:else}
							<select bind:value={branchOrIterationN} class="!w-32 grow">
								{#each restartBranchNames as [branchIdx, branchName] (branchIdx)}
									<option value={branchIdx}>{branchName}</option>
								{/each}
							</select>
						{/if}
					</div>
				</label>

				{#if flowPath}
					{@render flowVersionSelector()}
				{/if}
				<Button variant="accent" onClick={handleRestart}>Restart</Button>
			</div>
		{/snippet}
	</Popover>
{/if}
