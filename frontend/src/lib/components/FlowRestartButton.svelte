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
		disabled = false,
		enterpriseOnly = false,
		variant = 'default',
		unifiedSize = 'md',
		onRestart,
		onRestartComplete
	}: Props = $props()

	let branchOrIterationN = $state(0)
	let selectedVersionMode: 'run' | 'custom' = $state('run')
	let customFlowVersion: number | undefined = $state(undefined)
	let flowVersions: Array<FlowVersion> = $state([])
	let loadingVersions: boolean = $state(false)

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
		if (!flowPath || loadingVersions) return
		loadingVersions = true
		try {
			flowVersions = await FlowService.getFlowHistory({
				workspace: $workspaceStore!,
				path: flowPath
			})
			if (flowVersions.length > 0 && customFlowVersion === undefined) {
				customFlowVersion = flowVersions[0].id
			}
		} catch (e) {
			sendUserToast('Failed to load flow versions', true)
		} finally {
			loadingVersions = false
		}
	}

	function getFlowVersionForRestart(): number | undefined {
		if (selectedVersionMode === 'run') {
			return undefined // use run version
		} else if (selectedVersionMode === 'custom') {
			return customFlowVersion
		}
		return undefined
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
		<div class="flex flex-col gap-2">
			<select
				bind:value={selectedVersionMode}
				class="grow"
				onchange={() => {
					if (selectedVersionMode === 'custom' && flowVersions.length === 0) {
						loadFlowVersions()
					}
				}}
			>
				<option value="run">Run version</option>
				<option value="custom">Specific version</option>
			</select>

			{#if selectedVersionMode === 'custom'}
				{#if loadingVersions}
					<div class="text-xs text-secondary">Loading versions...</div>
				{:else if flowVersions.length > 0}
					<select bind:value={customFlowVersion} class="grow text-xs">
						{#each flowVersions as version}
							<option value={version.id}>
								{#if emptyString(version.deployment_msg)}Version {version.id}{:else}{version.deployment_msg}{/if}
								- {new Date(version.created_at).toLocaleString()}
							</option>
						{/each}
					</select>
				{:else}
					<div class="text-xs text-tertiary">No versions available</div>
				{/if}
			{/if}
		</div>
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
		>
			{#snippet trigger()}
				{@render singleRestartButton()}
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col gap-4 text-primary p-4 min-w-64">
					{@render flowVersionSelector()}

					<Button variant="accent" onClick={handleRestart}>Restart</Button>
				</div>
			{/snippet}
		</Popover>
	{/if}
{:else}
	<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}>
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
			<div class="flex flex-col gap-4 text-primary p-4 min-w-64">
				<label>
					<div class="pb-1 text-xs font-semibold text-emphasis"
						>{selectedJobStepType == 'forloop' ? 'From iteration #' : 'From branch'}</div
					>
					<div class="flex w-full gap-2">
						{#if selectedJobStepType === 'forloop'}
							<input type="number" min="0" bind:value={branchOrIterationN} class="!w-32 grow" />
						{:else}
							<select bind:value={branchOrIterationN} class="!w-32 grow">
								{#each restartBranchNames as [branchIdx, branchName]}
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
