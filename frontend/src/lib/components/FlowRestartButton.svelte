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
		/**
		 * For nested-step restarts: path of ancestor containers from the top-level
		 * step down to the leaf. When provided, the LAST entry's step_id is the
		 * actual restart point and `selectedJobStep` (the leaf shown in the UI) is
		 * informational only — the request is built from `nestedTopStepId`,
		 * `nestedTopBranchOrIterationN` and `nestedPath`.
		 */
		nestedPath?: Array<{ step_id: string; branch_or_iteration_n?: number }>
		/** Top-level container step id (only used when nestedPath is set) */
		nestedTopStepId?: string
		/** Top-level container branch_or_iteration_n (only used when nestedPath is set) */
		nestedTopBranchOrIterationN?: number
		/**
		 * For top-level ForLoop restart: iteration index the user is currently
		 * viewing in the graph (read from `selectedForloopIndex` upstream). When
		 * provided, the popup pre-fills the iteration selector with this value.
		 */
		presetIterationN?: number
		/**
		 * Map from ForLoop step id to the number of iterations that ran in the
		 * original execution (i.e. `flow_jobs.length`). When provided for a step,
		 * the popup renders a `<select>` of `0..count-1` instead of a free-form
		 * number input — same surface as the graph's iteration tabs. Used for the
		 * SELECTED step's iteration picker when it is itself a top-level ForLoop.
		 */
		iterationCounts?: Record<string, number>
		/**
		 * Iteration counts for nested-path entries, keyed by the popup's field-key
		 * (`'top'` for the outer container, `'inner-N'` for nested ancestors).
		 * Populated by the composable so each entry uses the *correct* graph-state
		 * key (prefixed for in-subflow ancestors), avoiding collisions when the
		 * same step id appears at multiple nesting levels.
		 */
		nestedPathIterationCounts?: Record<string, number>
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
		nestedPath = undefined,
		nestedTopStepId = undefined,
		nestedTopBranchOrIterationN = undefined,
		presetIterationN = undefined,
		iterationCounts = undefined,
		nestedPathIterationCounts = undefined,
		onRestart,
		onRestartComplete
	}: Props = $props()

	const isNested = $derived((nestedPath?.length ?? 0) > 0)
	// Inline-expanded subflow steps come in as `subflow:A:B:leaf` — show only `leaf`.
	const displayStepId = $derived(
		isNested && nestedPath && nestedPath.length > 0
			? nestedPath[nestedPath.length - 1].step_id
			: selectedJobStep
	)

	// Sentinel value meaning "use the same version as the original run" (backend receives undefined)
	const RUN_VERSION_SENTINEL = -1

	let branchOrIterationN = $state(0)

	// Iteration inputs for nested-step restart inside one or more ForLoop ancestors.
	// Each editable position keeps a local copy that the popup can mutate; on
	// submit we rebuild the request from these. We snapshot from props every time
	// the popup opens (see `resetEditsFromProps`), not on every prop change — that
	// way the user's manual edits stick while the popup is open, and re-opening
	// after picking a different iteration in the graph picks up the new value.
	type IterField = { key: 'top' | `inner-${number}`; label: string; value: number }
	let iterationEdits: Record<string, number> = $state({})

	function resetEditsFromProps() {
		const next: Record<string, number> = {}
		if (isNested && nestedTopBranchOrIterationN !== undefined) {
			next['top'] = nestedTopBranchOrIterationN
		}
		if (isNested) {
			nestedPath?.forEach((entry, i) => {
				if (entry.branch_or_iteration_n !== undefined) {
					next[`inner-${i}`] = entry.branch_or_iteration_n
				}
			})
		}
		iterationEdits = next
		if (selectedJobStepType === 'forloop' && presetIterationN !== undefined) {
			branchOrIterationN = presetIterationN
		}
	}
	const iterationFields = $derived.by((): IterField[] => {
		if (!isNested) return []
		const out: IterField[] = []
		if (nestedTopBranchOrIterationN !== undefined && nestedTopStepId) {
			out.push({
				key: 'top',
				label: nestedTopStepId,
				value: iterationEdits['top'] ?? nestedTopBranchOrIterationN
			})
		}
		nestedPath?.forEach((entry, i) => {
			if (entry.branch_or_iteration_n !== undefined) {
				const k = `inner-${i}` as const
				out.push({
					key: k,
					label: entry.step_id,
					value: iterationEdits[k] ?? entry.branch_or_iteration_n
				})
			}
		})
		return out
	})
	const needsPopup = $derived(
		!!flowPath || iterationFields.length > 0 || selectedJobStepType !== 'single'
	)
	let selectedFlowVersion: number = $state(RUN_VERSION_SENTINEL)
	let flowVersions: Array<FlowVersion> = $state([])
	let loadingVersions: boolean = $state(false)
	let versionsLoaded: boolean = $state(false)
	let runVersionInList: boolean = $state(false)

	async function restartFlow(stepId: string, branchOrIterationN: number, flowVersion?: number) {
		const requestBody = isNested
			? {
					step_id: nestedTopStepId!,
					branch_or_iteration_n: iterationEdits['top'] ?? nestedTopBranchOrIterationN,
					flow_version: flowVersion,
					nested_path: nestedPath?.map((entry, i) => ({
						step_id: entry.step_id,
						branch_or_iteration_n: iterationEdits[`inner-${i}`] ?? entry.branch_or_iteration_n
					}))
				}
			: {
					step_id: stepId,
					branch_or_iteration_n: branchOrIterationN,
					flow_version: flowVersion
				}
		let run = await JobService.restartFlowAtStep({
			workspace: $workspaceStore!,
			id: jobId,
			requestBody
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
{#snippet restartTriggerButton(usePlayIcon: boolean)}
	<Button
		title={`Re-start this flow from step ${displayStepId} (included).${enterpriseOnly ? ' This is a feature only available in enterprise edition.' : ''}`}
		{variant}
		{unifiedSize}
		{disabled}
		startIcon={{ icon: usePlayIcon ? Play : RefreshCw }}
		nonCaptureEvent={!usePlayIcon || !!flowPath}
		onClick={() => {
			if (usePlayIcon && !flowPath) {
				handleRestart()
			}
		}}
	>
		Re-start from
		<Badge baseClass="ml-1" color="indigo">
			{displayStepId}
		</Badge>
		{#if enterpriseOnly && disabled}
			(EE)
		{/if}
	</Button>
{/snippet}

{#if !needsPopup}
	{@render restartTriggerButton(true)}
{:else}
	<Popover
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
		disablePopup={!needsPopup}
		on:openChange={(e) => {
			if (e.detail) {
				loadFlowVersions()
				resetEditsFromProps()
			}
		}}
	>
		{#snippet trigger()}
			{@render restartTriggerButton(selectedJobStepType === 'single')}
		{/snippet}
		{#snippet content()}
			<div class="flex flex-col gap-4 text-primary p-4 w-80">
				{#if selectedJobStepType === 'forloop'}
					{@const topCount = iterationCounts?.[selectedJobStep] ?? 0}
					<label>
						<div class="pb-1 text-xs font-semibold text-emphasis">From iteration #</div>
						<div class="flex w-full gap-2">
							{#if topCount > 0}
								<select bind:value={branchOrIterationN} class="!w-32 grow">
									{#each Array.from({ length: topCount }, (_, i) => i) as i (i)}
										<option value={i}>{i + 1}</option>
									{/each}
								</select>
							{:else}
								<input type="number" min="0" bind:value={branchOrIterationN} class="!w-32 grow" />
							{/if}
						</div>
					</label>
				{:else if selectedJobStepType === 'branchall'}
					<label>
						<div class="pb-1 text-xs font-semibold text-emphasis">From branch</div>
						<div class="flex w-full gap-2">
							<select bind:value={branchOrIterationN} class="!w-32 grow">
								{#each restartBranchNames as [branchIdx, branchName] (branchIdx)}
									<option value={branchIdx}>{branchName}</option>
								{/each}
							</select>
						</div>
					</label>
				{/if}

				{#each iterationFields as field (field.key)}
					{@const count = nestedPathIterationCounts?.[field.key] ?? 0}
					<label>
						<div class="pb-1 text-xs font-semibold text-emphasis">
							From iteration # of <code class="text-xs">{field.label}</code>
						</div>
						<div class="flex w-full gap-2">
							{#if count > 0}
								<select
									value={field.value}
									onchange={(e) => {
										const v = parseInt((e.target as HTMLSelectElement).value, 10)
										if (!Number.isNaN(v)) {
											iterationEdits = { ...iterationEdits, [field.key]: v }
										}
									}}
									class="!w-32 grow"
								>
									{#each Array.from({ length: count }, (_, i) => i) as i (i)}
										<option value={i}>{i + 1}</option>
									{/each}
								</select>
							{:else}
								<input
									type="number"
									min="0"
									value={field.value}
									oninput={(e) => {
										const v = parseInt((e.target as HTMLInputElement).value, 10)
										if (!Number.isNaN(v)) {
											iterationEdits = { ...iterationEdits, [field.key]: v }
										}
									}}
									class="!w-32 grow"
								/>
							{/if}
						</div>
					</label>
				{/each}

				{#if flowPath}
					{@render flowVersionSelector()}
				{/if}
				<Button variant="accent" onClick={handleRestart}>Restart</Button>
			</div>
		{/snippet}
	</Popover>
{/if}
