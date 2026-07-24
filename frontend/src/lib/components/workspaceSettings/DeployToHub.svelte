<script lang="ts">
	import { base } from '$lib/base'
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import WorkspaceDeployLayout from '$lib/components/WorkspaceDeployLayout.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore, hubBaseUrlStore, enterpriseLicense } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import {
		useDeployToHubSession,
		canRecord,
		canShareAsIframe,
		sanitizeSlug,
		isValidSlug,
		type DeployItem
	} from './deployToHubSession.svelte'
	import { TRIGGER_KINDS, triggerDetails } from '$lib/components/triggers/workspaceTriggersList'
	import Toggle from '../Toggle.svelte'
	import MigrationSqlEditor from './MigrationSqlEditor.svelte'
	import PipelineRecordingReplay from '$lib/components/recording/PipelineRecordingReplay.svelte'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import {
		Check,
		Cloud,
		Code2,
		Copy,
		Database,
		Eye,
		ExternalLink,
		Globe,
		Info,
		LayoutDashboard,
		Loader2,
		Play,
		RotateCcw,
		TriangleAlert,
		X,
		Zap
	} from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import Popover from '$lib/components/Popover.svelte'

	// Folder name (no `f/`) this project is scoped to; provided by the /folders launcher.
	let { folder: folderProp }: { folder: string } = $props()

	// All deploy state lives in a session keyed by (workspace, folder): switching
	// either replaces the instance (and the {#key} below remounts the UI, closing
	// drawers), so nothing here needs resetting by hand.
	const deployHub = useDeployToHubSession({
		workspace: () => $workspaceStore,
		folder: () => folderProp,
		hasEeLicense: () => !!$enterpriseLicense
	})

	let recordDrawer = $state<Drawer | undefined>()
	let pipelinePreviewDrawer = $state<Drawer | undefined>()
	let pipelineGraphDrawer = $state<Drawer | undefined>()
	let publishDrawer = $state<Drawer | undefined>()
	let resourceDrawer = $state<Drawer | undefined>()
	let triggerDrawer = $state<Drawer | undefined>()
	let bundleDrawer = $state<Drawer | undefined>()

	let hubUrl = $derived(
		`${$hubBaseUrlStore.replace(/\/+$/, '')}/projects/${deployHub.session?.hubSlug ?? ''}`
	)

	// The session is UI-free; the component owns drawer open/close around its
	// operations.
	function openBundle() {
		const s = deployHub.session
		if (!s || s.triggersLoading) return
		s.prepareBundle()
		bundleDrawer?.openDrawer()
	}
	async function confirmBundle() {
		await deployHub.session?.publishBundle(() => bundleDrawer?.closeDrawer())
	}
	function openRecord(it: DeployItem) {
		recordDrawer?.openDrawer()
		void deployHub.session?.openRecord(it)
	}
	async function saveRecording() {
		if (await deployHub.session?.saveRecording()) recordDrawer?.closeDrawer()
	}
	async function savePipelineRecording() {
		await deployHub.session?.savePipelineRecording()
	}
	function openPublish(it: DeployItem) {
		const s = deployHub.session
		if (!s) return
		s.publishTarget = it
		publishDrawer?.openDrawer()
	}
	async function confirmPublish() {
		if (await deployHub.session?.confirmPublish()) publishDrawer?.closeDrawer()
	}
	async function copyIframeSnippet(url: string) {
		const snippet = `<iframe src="${url}" width="100%" height="600" frameborder="0"></iframe>`
		try {
			await navigator.clipboard.writeText(snippet)
			sendUserToast('Iframe snippet copied to clipboard')
		} catch {
			sendUserToast('Failed to copy snippet', true)
		}
	}
</script>

{#if deployHub.session}
	{#key deployHub.session}
		{@const s = deployHub.session}
		<div>
			<WorkspaceDeployLayout
				items={s.items}
				selectedItems={s.selectedItemKeys}
				deploymentStatus={s.deploymentStatus}
				hideSelection={s.phase !== 'predeploy'}
				allSelected={s.allSelected}
				onToggleItem={s.toggleItem}
				onSelectAll={s.selectAll}
				onDeselectAll={s.deselectAll}
				emptyMessage={s.loading ? 'Loading project items…' : 'No items to publish'}
			>
				{#snippet header()}
					{@const stepNum =
						s.phase === 'predeploy'
							? 1
							: s.phase === 'draft'
								? 2
								: s.phase === 'under_review'
									? 3
									: 4}
					<div class="flex flex-col gap-2 w-full pb-4">
						<ol
							class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3 text-xs text-secondary"
						>
							<span class="text-sm font-semibold text-primary">
								How to publish your project to the Hub
							</span>
							<li class={stepNum === 1 ? 'text-primary' : stepNum > 1 ? 'opacity-60' : ''}>
								<span class="font-mono text-emphasis">{stepNum > 1 ? '✓' : '1.'}</span>
								<span class="font-semibold text-primary">Bundle your project</span> — creates a draft
								on the Hub with every selected script, flow, app and resource from this folder.
							</li>
							<li
								class={stepNum === 2 ? 'text-primary' : stepNum > 2 ? 'opacity-60' : 'opacity-40'}
							>
								<span class="font-mono text-emphasis">{stepNum > 2 ? '✓' : '2.'}</span>
								<span class="font-semibold text-primary">Generate iframes &amp; recordings</span> — share
								public apps as iframes, capture one execution per script/flow, and record the whole data-pipeline
								cascade as one interactive replay.
							</li>
							<li
								class={stepNum === 3 ? 'text-primary' : stepNum > 3 ? 'opacity-60' : 'opacity-40'}
							>
								<span class="font-mono text-emphasis">{stepNum > 3 ? '✓' : '3.'}</span>
								<span class="font-semibold text-primary">Submit for review</span> — send the bundle for
								approval.
							</li>
						</ol>
						<div class="flex flex-wrap items-center gap-2 pt-4">
							{#if s.phase === 'predeploy'}
								<span class="text-sm font-semibold text-primary">
									Step 1: Bundle your project
								</span>
							{:else if s.phase === 'draft'}
								<span class="text-sm font-semibold text-primary">
									Step 2: Generate iframes &amp; recordings
								</span>
							{:else if s.phase === 'under_review'}
								<span class="text-sm font-semibold text-primary">Step 3: Awaiting review</span>
							{:else}
								<span class="text-sm font-semibold text-primary">Live on the Hub</span>
							{/if}
							{#if s.phase !== 'predeploy'}
								<Badge color="transparent" class="font-semibold">
									<Cloud size={14} class="mr-1" />
									<span class="text-secondary">on Hub:</span>
									<span class="text-emphasis">{s.hubName || s.hubSlug}</span>
								</Badge>
								<a
									href={hubUrl}
									target="_blank"
									rel="noopener noreferrer"
									class="inline-flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400 hover:underline"
								>
									<ExternalLink size={12} /> Open in Hub
								</a>
							{/if}
							<div class="ml-auto flex items-center gap-2">
								{#if s.phase === 'predeploy'}
									<Button
										variant="accent"
										loading={s.deploying || s.triggersLoading}
										disabled={s.selectedItems.length === 0 ||
											s.triggersLoading ||
											s.triggerDiscoveryFailed}
										startIcon={{ icon: Cloud }}
										onclick={openBundle}
									>
										Create Hub draft ({s.selectedItems.length})
									</Button>
								{:else if s.phase === 'draft'}
									<Button
										variant="accent"
										loading={s.submitting}
										startIcon={{ icon: Check }}
										onclick={s.submitForReview}
									>
										Submit for review
									</Button>
								{:else if s.phase === 'under_review'}
									<Button
										size="xs"
										variant="subtle"
										loading={s.syncing}
										startIcon={{ icon: RotateCcw }}
										iconOnly
										title="Refresh review status"
										onclick={s.syncWithHub}
									/>
								{:else}
									<Button
										variant="accent"
										startIcon={{ icon: RotateCcw }}
										onclick={s.startNewDraft}
									>
										New draft
									</Button>
								{/if}
							</div>
						</div>
						{#if s.phase === 'predeploy'}
							<div class="flex flex-col gap-1 pb-3">
								<span class="text-xs text-secondary">
									Bundling creates a draft project on the Hub from the selected scripts, flows and
									apps of <span class="font-mono">{s.selectedFolder}/</span>.
									{s.selectedItems.length} of {s.filteredWorkspaceItems.length} items selected.
								</span>
							</div>
							<div class="flex flex-wrap items-center gap-2 text-xs">
								<span class="font-semibold text-primary shrink-0">
									Resource dependencies
									{#if s.detectingResources}
										<Loader2 size={11} class="inline animate-spin text-hint" />
									{:else}
										<span class="text-hint font-normal">({s.dependencyTypes.length})</span>
									{/if}
									<Tooltip>
										Resource types the selected items depend on (whether passed as inputs or
										referenced by a hardcoded path). Synced to the Hub so a fork knows what
										credentials it needs to fill.
									</Tooltip>
								</span>
								{#if s.dependencyTypes.length === 0}
									<span class="text-[11px] text-hint">
										No resource references detected in the current selection.
									</span>
								{:else}
									{#each s.dependencyTypes as r (r.resource_type)}
										<span
											class="inline-flex items-center gap-1 rounded border px-1.5 py-0.5 font-mono text-[11px] text-secondary {r.hasHardcoded
												? 'border-amber-300 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/40'
												: 'bg-surface'}"
										>
											{r.resource_type}
										</span>
									{/each}
									<Button
										variant="subtle"
										unifiedSize="sm"
										wrapperClasses="ml-auto"
										onclick={() => resourceDrawer?.openDrawer()}
									>
										View details
									</Button>
								{/if}
							</div>
							<div class="flex flex-wrap items-center gap-2 text-xs">
								<span class="font-semibold text-primary shrink-0">
									Data table dependencies
									{#if s.detectingDatatables}
										<Loader2 size={11} class="inline animate-spin text-hint" />
									{:else}
										<span class="text-hint font-normal">({s.datatableUsage.size})</span>
									{/if}
									<Tooltip>
										Data tables the selected items read or write. A best-effort CREATE TABLE
										migration for these is generated in the bundle step and shipped with the
										project, so a fork can recreate the tables it needs.
									</Tooltip>
								</span>
								{#if s.datatableUsage.size === 0}
									<span class="text-[11px] text-hint">
										No data table usage detected in the current selection.
									</span>
								{:else}
									{#each [...s.datatableUsage] as [dt, tables] (dt)}
										<span
											class="inline-flex items-center gap-1 rounded border bg-surface px-1.5 py-0.5 font-mono text-[11px] text-secondary"
										>
											{dt}
											{#if tables.size > 0}
												<span class="text-hint">×{tables.size}</span>
											{/if}
										</span>
									{/each}
								{/if}
							</div>
							{#if s.isPipelineProject}
								<div class="flex flex-wrap items-center gap-2 text-xs">
									<span class="font-semibold text-primary shrink-0">
										Data pipeline
										<span class="text-hint font-normal">
											({s.pipelineScriptPaths.length} step{s.pipelineScriptPaths.length === 1
												? ''
												: 's'})
										</span>
										<Tooltip>
											Scripts in this folder form a data-pipeline cascade (each reads or writes data
											tables consumed by the next). Bundle the whole folder, then record the cascade
											as one interactive replay in step 2.
										</Tooltip>
									</span>
									<Button
										variant="subtle"
										unifiedSize="sm"
										startIcon={{ icon: BarsStaggered }}
										wrapperClasses="ml-auto"
										onclick={() => pipelineGraphDrawer?.openDrawer()}
									>
										View pipeline graph
									</Button>
								</div>
							{/if}
						{/if}
						{#if s.phase === 'draft'}
							<div class="flex flex-col gap-1 pb-3">
								<span class="text-xs text-secondary">
									A recording captures one real run of a script or flow — inputs, logs, step outputs
									and result — replayable on the Hub so visitors see it work before forking. Public
									apps can also be shared as live iframes. Optional, but recommended.
								</span>
							</div>
						{/if}
						{#if s.phase === 'draft' && s.isPipelineProject}
							<div
								class="flex flex-col gap-2 rounded-md border border-blue-300 bg-blue-50 p-3 dark:border-blue-800 dark:bg-blue-950/40"
							>
								<div class="flex flex-wrap items-center gap-2">
									<BarsStaggered class="text-blue-600 dark:text-blue-400" />
									<span class="text-sm font-semibold text-primary">Data pipeline recording</span>
									{#if s.pipelineRecorded}
										<Badge color="green" size="xs">
											<Check size={10} class="mr-0.5" />Recorded
										</Badge>
									{/if}
									<div class="ml-auto flex items-center gap-2">
										<Button
											size="xs"
											variant="subtle"
											loading={s.pipelineRunState === 'running'}
											startIcon={{ icon: s.pipelineRecordingResult ? RotateCcw : Play }}
											onclick={() => s.runPipelineRecording()}
										>
											{s.pipelineRecordingResult ? 'Re-run' : 'Record pipeline run'}
										</Button>
										{#if s.pipelineRecordingResult}
											<Button
												size="xs"
												variant="subtle"
												startIcon={{ icon: Eye }}
												onclick={() => pipelinePreviewDrawer?.openDrawer()}
											>
												Preview
											</Button>
											<Button
												size="xs"
												variant="accent"
												disabled={s.pipelineRunState !== 'success'}
												startIcon={{ icon: Check }}
												onclick={savePipelineRecording}
											>
												Save as recording
											</Button>
										{/if}
									</div>
								</div>
								<span class="text-xs text-secondary">
									Runs this project's <span class="font-mono">{s.selectedFolder}/</span> pipeline
									cascade ({s.recordablePipelineScriptPaths.length} step{s
										.recordablePipelineScriptPaths.length === 1
										? ''
										: 's'}) and captures the asset graph, per-step logs/results and table samples
									into one interactive replay for the project page.
								</span>
								{#if s.pipelineRunState === 'running'}
									<div class="flex items-center gap-2 text-xs text-secondary">
										<Loader2 size={12} class="animate-spin" /> Running the pipeline cascade…
									</div>
								{:else if s.pipelineRunState === 'success'}
									<div class="flex items-center gap-2 text-xs text-green-700 dark:text-green-400">
										<Check size={12} /> Cascade succeeded — preview it, then save as the recording.
									</div>
								{:else if s.pipelineRunState === 'failed'}
									<div class="flex items-center gap-2 text-xs text-red-600 dark:text-red-400">
										<TriangleAlert size={12} />
										{s.pipelineRunError ?? 'Cascade failed'}
									</div>
								{/if}
							</div>
						{/if}
						{#if s.phase === 'predeploy'}
							<div class="flex flex-wrap items-center gap-2 text-xs">
								<span class="font-semibold text-primary shrink-0">
									Triggers
									{#if s.triggersLoading}
										<Loader2 size={11} class="inline animate-spin text-hint" />
									{:else}
										<span class="text-hint font-normal">({s.relevantTriggers.length})</span>
									{/if}
								</span>
								{#if s.triggerDiscoveryFailed}
									<span class="text-[11px] text-red-600 dark:text-red-400">
										Some trigger kinds could not be listed — publishing is disabled so triggers
										aren't silently left out of the bundle.
									</span>
									<Button
										size="xs"
										variant="subtle"
										loading={s.triggersLoading}
										disabled={s.triggersLoading}
										startIcon={{ icon: RotateCcw }}
										onclick={() => s.reloadTriggers()}
									>
										Retry
									</Button>
								{:else if s.relevantTriggers.length === 0}
									<span class="text-[11px] text-hint"
										>No triggers reference the selected items.</span
									>
								{:else}
									{#each s.triggersByKind as [kind, triggers] (kind)}
										<span
											class="inline-flex items-center gap-1 rounded border bg-surface px-1.5 py-0.5 font-mono text-[11px] text-secondary"
										>
											{TRIGGER_KINDS[kind].badge}
											<span class="text-hint">×{triggers.length}</span>
										</span>
									{/each}
									<Button
										variant="subtle"
										unifiedSize="sm"
										wrapperClasses="ml-auto"
										onclick={() => triggerDrawer?.openDrawer()}
									>
										View details
									</Button>
								{/if}
							</div>
						{/if}
						{#if s.phase === 'under_review'}
							<div
								class="flex items-start gap-2 rounded-md border border-blue-300 bg-blue-50 p-3 text-xs text-blue-900 dark:border-blue-800 dark:bg-blue-950/40 dark:text-blue-100"
							>
								<TriangleAlert size={14} class="mt-0.5 shrink-0 text-blue-600 dark:text-blue-400" />
								<div class="flex flex-col gap-1">
									<span class="font-semibold">Locked while under review</span>
									<span>
										The Windmill team is reviewing this submission. Editing, recording, and sharing
										actions are disabled. Estimated turnaround: 1-2 business days.
									</span>
								</div>
							</div>
						{/if}
						{#if s.phase === 'draft'}
							{@const recordedCount = s.recordableItems.filter((i) => i.rec === 'recorded').length}
							{@const pct = s.recordableItems.length
								? Math.round((recordedCount / s.recordableItems.length) * 100)
								: 0}
							<div class="flex items-center gap-2 self-end text-[11px] text-tertiary">
								<span class="font-mono">{recordedCount}/{s.recordableItems.length}</span>
								<div class="h-1 w-24 overflow-hidden rounded bg-surface-tertiary">
									<div
										class="h-full {s.allRecorded ? 'bg-green-500' : 'bg-hint'} transition-all"
										style="width: {pct}%"
									></div>
								</div>
								<span class={s.allRecorded ? 'text-green-700 dark:text-green-400' : 'text-hint'}>
									{s.allRecorded ? 'Full recordings' : 'Recordings recommended'}
								</span>
							</div>
						{/if}
					</div>
				{/snippet}

				{#snippet itemSummary(item)}
					{@const it = item as DeployItem}
					<span class="flex min-w-0 items-center gap-1.5">
						<span class="truncate">
							{it.summary?.trim() || it.path}
						</span>
						{#if it.kind === 'script' && s.pipelineScriptPathSet.has(it.path)}
							<Badge color="blue" size="xs" wrapperClass="shrink-0">
								<BarsStaggered size={10} class="mr-0.5" />Pipeline
							</Badge>
						{/if}
					</span>
				{/snippet}

				{#snippet itemActions(item)}
					{@const it = item as DeployItem}
					{#if s.phase !== 'predeploy' && canRecord(it.kind)}
						{#if it.rec === 'recorded'}
							<Badge color="green" size="xs">
								<Check size={10} class="mr-0.5" />Recorded
							</Badge>
							{#if s.recordings[it.key]}
								<a
									href={`/run/${s.recordings[it.key]}?workspace=${s.workspace}`}
									target="_blank"
									rel="noopener noreferrer"
									class="inline-flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400 hover:underline"
								>
									<ExternalLink size={12} /> See recording
								</a>
							{/if}
							{#if s.phase === 'draft'}
								<Button
									size="xs"
									variant="subtle"
									startIcon={{ icon: RotateCcw }}
									onclick={() => openRecord(it)}
								>
									Re-record
								</Button>
							{/if}
						{:else if s.phase === 'draft'}
							<Badge color="yellow" size="xs">No recording</Badge>
							<Button
								size="xs"
								variant="subtle"
								startIcon={{ icon: Play }}
								onclick={() => openRecord(it)}
							>
								Add recording
							</Button>
						{:else}
							<Badge color="yellow" size="xs">No recording</Badge>
						{/if}
					{/if}
					{#if s.phase !== 'predeploy' && canShareAsIframe(it)}
						{#if it.published}
							<Badge color="green" size="xs">
								<Globe size={10} class="mr-0.5" />Public
							</Badge>
							{#if it.publicUrl}
								<a
									href={it.publicUrl}
									target="_blank"
									rel="noopener noreferrer"
									class="inline-flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400 hover:underline"
								>
									<ExternalLink size={12} /> Open
								</a>
								<Button
									size="xs"
									variant="subtle"
									startIcon={{ icon: Copy }}
									onclick={() => copyIframeSnippet(it.publicUrl!)}
								>
									Copy iframe
								</Button>
							{:else if s.phase !== 'under_review'}
								<!-- Public but its URL didn't resolve: offer a retry, keep Unpublish. -->
								<Button
									size="xs"
									variant="subtle"
									startIcon={{ icon: RotateCcw }}
									onclick={() => openPublish(it)}
								>
									Retry link
								</Button>
							{/if}
							{#if s.phase !== 'under_review'}
								<Button size="xs" variant="subtle" onclick={() => s.unpublishApp(it)}
									>Unpublish</Button
								>
							{/if}
						{:else if s.phase !== 'under_review'}
							<Button
								size="xs"
								variant="subtle"
								startIcon={{ icon: Globe }}
								onclick={() => openPublish(it)}
							>
								Share as iframe
							</Button>
						{/if}
					{/if}
				{/snippet}

				{#snippet footer()}
					<div class="flex items-center justify-end gap-3">
						{#if s.phase === 'predeploy'}
							<span class="text-[11px] text-hint">
								Select the items to include — all selected by default.
							</span>
						{:else if s.phase === 'draft'}
							<span class="text-[11px] text-hint">
								{#if s.allRecorded}
									All scripts and flows have a recording — best chance of approval and featuring.
								{:else}
									{s.recordableItems.filter((i) => i.rec === 'recorded').length} of {s
										.recordableItems.length}
									recorded. Bundles with full recordings get approved faster and featured on the public
									Hub.
								{/if}
							</span>
						{:else if s.phase === 'under_review'}
							<span class="text-[11px] text-hint">
								Waiting for the Windmill team to review the submission.
							</span>
						{:else}
							<span class="text-[11px] text-hint"> Iterate further by starting a new draft. </span>
						{/if}
					</div>
				{/snippet}
			</WorkspaceDeployLayout>
		</div>

		<Drawer bind:this={recordDrawer} size="600px" on:close={s.cancelRecordRun}>
			<DrawerContent
				title={s.recordTarget ? `Record — ${s.recordTarget.path}` : 'Record'}
				on:close={() => recordDrawer?.closeDrawer()}
			>
				<div class="flex flex-col gap-3">
					<p class="text-xs text-secondary">
						Run this {s.recordTarget?.kind} once with the inputs below. The full execution — args, logs,
						intermediate step outputs and final result — is saved as a <b>replayable recording</b>
						shown on the Hub page. Visitors can step through it to see how the {s.recordTarget
							?.kind} works without running anything themselves.
					</p>

					{#if s.runState !== 'idle'}
						<div
							class="sticky top-0 z-10 flex flex-col gap-2 rounded-md border bg-surface-secondary p-3 shadow-sm"
						>
							<div class="flex items-center gap-2 text-sm">
								{#if s.runState === 'running'}
									<Loader2 size={14} class="animate-spin text-blue-600 dark:text-blue-400" />
									<span class="font-semibold">Running…</span>
								{:else if s.runState === 'success'}
									<Check size={14} class="text-green-600 dark:text-green-400" />
									<span class="font-semibold text-green-700 dark:text-green-300">
										Execution succeeded
									</span>
								{:else}
									<X size={14} class="text-red-600 dark:text-red-400" />
									<span class="font-semibold text-red-700 dark:text-red-300">Execution failed</span>
								{/if}
								{#if s.runJobId}
									<a
										href={`/run/${s.runJobId}?workspace=${s.workspace}`}
										target="_blank"
										rel="noopener noreferrer"
										class="inline-flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400 hover:underline"
									>
										<ExternalLink size={12} /> Open job
									</a>
								{/if}
							</div>
							{#if s.runState === 'success' && s.runResult !== undefined}
								<div class="flex flex-col gap-1 text-xs">
									<span class="text-secondary">Result preview:</span>
									<pre class="max-h-40 overflow-auto rounded bg-surface p-2 font-mono text-[11px]"
										>{JSON.stringify(s.runResult, null, 2)}</pre
									>
								</div>
							{:else if s.runState === 'failed' && s.runError}
								<pre
									class="max-h-40 overflow-auto rounded bg-surface p-2 font-mono text-[11px] text-red-700 dark:text-red-300"
									>{s.runError}</pre
								>
							{/if}
							{#if s.runState === 'success'}
								<div
									class="mt-1 flex items-center justify-between gap-3 rounded-md border border-green-300 bg-green-50 p-2 dark:border-green-800 dark:bg-green-950/40"
								>
									<span class="text-xs text-green-900 dark:text-green-100">
										Looks good? Save this run as the Hub recording.
									</span>
									<Button
										size="xs"
										variant="accent"
										startIcon={{ icon: Check }}
										onclick={saveRecording}
									>
										Save as recording
									</Button>
								</div>
							{:else if s.runState === 'failed'}
								<span class="text-[11px] text-hint">
									Fix inputs and try again. Only successful runs can be saved as a recording.
								</span>
							{/if}
						</div>
					{/if}

					{#if s.recordSchemaLoading}
						<span class="text-xs text-hint">Loading schema…</span>
					{:else}
						<SchemaForm
							bind:args={s.recordArgs}
							bind:isValid={s.recordValid}
							schema={s.recordSchema}
						/>
					{/if}
				</div>
				{#snippet actions()}
					{#if s.runState === 'success'}
						<Button variant="default" startIcon={{ icon: RotateCcw }} onclick={s.runJob}
							>Re-run</Button
						>
						<Button variant="accent" startIcon={{ icon: Check }} onclick={saveRecording}>
							Save as recording
						</Button>
					{:else}
						<Button
							variant="accent"
							loading={s.runState === 'running'}
							disabled={!s.recordValid || s.recordSchemaLoading}
							startIcon={{ icon: Play }}
							onclick={s.runJob}
						>
							{s.runState === 'failed' ? 'Re-run' : 'Run'}
						</Button>
					{/if}
				{/snippet}
			</DrawerContent>
		</Drawer>

		<Drawer bind:this={pipelinePreviewDrawer} size="1100px">
			<DrawerContent
				title="Pipeline recording preview"
				on:close={() => pipelinePreviewDrawer?.closeDrawer()}
			>
				{#if s.pipelineRecordingResult}
					<div class="h-full min-h-[600px]">
						<PipelineRecordingReplay recording={s.pipelineRecordingResult} />
					</div>
				{/if}
			</DrawerContent>
		</Drawer>

		<Drawer bind:this={pipelineGraphDrawer} size="1100px">
			<DrawerContent title="Data pipeline" on:close={() => pipelineGraphDrawer?.closeDrawer()}>
				<div class="flex h-full flex-col gap-3 min-h-0">
					<p class="text-xs text-secondary shrink-0">
						The <span class="font-mono">{s.selectedFolder}/</span> scripts and the data tables they read
						and write, as a single pipeline. Click a node to inspect it. This whole cascade can be captured
						as an interactive replay once the project is bundled.
					</p>
					{#if s.pipelineGraph}
						<div class="flex-1 min-h-[600px] rounded-md border bg-surface overflow-hidden">
							<AssetGraphCanvas graph={s.pipelineGraph} viewportFitKey={s.folder} />
						</div>
					{:else}
						<span class="text-xs text-hint">Loading pipeline graph…</span>
					{/if}
				</div>
			</DrawerContent>
		</Drawer>

		<Drawer bind:this={publishDrawer} size="600px">
			<DrawerContent
				title={s.publishTarget ? `Share as iframe — ${s.publishTarget.path}` : 'Share as iframe'}
				on:close={() => publishDrawer?.closeDrawer()}
			>
				<div class="flex flex-col gap-4">
					<p class="text-xs text-secondary">
						Expose <span class="font-mono text-emphasis">{s.publishTarget?.path}</span> at a public URL
						so it can be embedded as an iframe (e.g. on the Hub, a docs page, or your own site). Anyone
						with the URL will be able to interact with it.
					</p>

					<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
						<div class="flex items-center gap-2">
							<TriangleAlert
								size={14}
								class={s.workspaceRateLimit
									? 'text-secondary'
									: 'text-orange-600 dark:text-orange-400'}
							/>
							<span class="text-sm font-semibold">Rate limit (workspace-wide)</span>
							<Tooltip>
								Caps public app executions per minute per server. Applies to all public apps in this
								workspace.
							</Tooltip>
						</div>
						{#if s.workspaceRateLimit && s.workspaceRateLimit > 0}
							<span class="text-xs text-secondary">
								Currently <span class="font-mono text-emphasis">{s.workspaceRateLimit}</span> executions
								/ minute / server.
							</span>
						{:else}
							<span class="text-xs text-orange-700 dark:text-orange-300">
								No rate limit configured — anyone with the URL can hit this app at any rate.
							</span>
						{/if}
						<a
							href="{base}/workspace_settings?tab=default_app"
							class="text-[11px] text-blue-600 underline"
							onclick={() => publishDrawer?.closeDrawer()}
						>
							Edit in Workspace settings → Apps
						</a>
					</div>
				</div>
				{#snippet actions()}
					<Button variant="default" onclick={() => publishDrawer?.closeDrawer()}>Cancel</Button>
					<Button
						variant="accent"
						loading={s.publishing}
						startIcon={{ icon: Globe }}
						onclick={confirmPublish}
					>
						Generate iframe
					</Button>
				{/snippet}
			</DrawerContent>
		</Drawer>

		<Drawer bind:this={resourceDrawer} size="640px">
			<DrawerContent title="Resource dependencies" on:close={() => resourceDrawer?.closeDrawer()}>
				<div class="flex flex-col gap-4">
					<p class="text-xs text-secondary">
						Resource types the selected items depend on. Each is synced to the Hub so a fork knows
						what credentials it needs to fill. <span class="font-semibold">Input</span> means the
						item takes the resource as a parameter;
						<span class="font-semibold">hardcoded path</span> means the item pins a specific resource
						path in its code.
					</p>
					{#if s.dependencyTypes.length === 0}
						<span class="text-xs text-hint">No resource references in the current selection.</span>
					{:else}
						{#each s.dependencyTypes as r (r.resource_type)}
							<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
								<div class="flex items-center gap-2 border-b pb-2">
									<span
										class="rounded border px-1.5 py-0.5 font-mono text-xs text-primary {r.hasHardcoded
											? 'border-amber-300 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/40'
											: 'bg-surface'}"
									>
										{r.resource_type}
									</span>
									<span class="text-[11px] text-hint">
										{r.usages.length} usage{r.usages.length > 1 ? 's' : ''}
									</span>
								</div>
								<div class="flex flex-col gap-3">
									{#each r.usages as u, ui (ui)}
										{#if u.role === 'trigger'}
											<div class="flex items-center gap-2 text-xs">
												<Zap size={14} class="shrink-0 text-hint" />
												<span class="break-all font-mono text-primary">{u.label}</span>
												<span
													class="ml-auto inline-flex shrink-0 items-center gap-1 rounded bg-surface px-1.5 py-px text-[10px] font-semibold uppercase tracking-wide text-secondary"
												>
													{TRIGGER_KINDS[u.triggerKind].badge} trigger
												</span>
											</div>
										{:else}
											{@const itemUrl = s.itemUrl(u.kind, u.itemPath)}
											<div class="flex flex-col gap-1 text-xs">
												<div class="flex items-center gap-2">
													{#if u.kind === 'script'}
														<Code2 size={14} class="shrink-0 text-hint" />
													{:else if u.kind === 'flow'}
														<BarsStaggered size={14} class="shrink-0 text-hint" />
													{:else}
														<LayoutDashboard size={14} class="shrink-0 text-hint" />
													{/if}
													<span class="break-all font-mono text-primary">{u.label}</span>
													<span
														class="ml-auto inline-flex shrink-0 items-center gap-1 rounded px-1.5 py-px text-[10px] font-semibold uppercase tracking-wide {u.role ===
														'hardcoded'
															? 'bg-amber-100 text-amber-800 dark:bg-amber-950/50 dark:text-amber-200'
															: 'bg-surface text-secondary'}"
													>
														{u.role === 'hardcoded' ? 'hardcoded path' : 'input'}
														{#if u.role === 'hardcoded'}
															<Popover notClickable placement="top">
																<Info size={11} class="text-amber-600 dark:text-amber-400" />
																{#snippet text()}
																	<div
																		class="flex w-80 max-w-[90vw] flex-col gap-2 text-left text-[11px] normal-case"
																	>
																		<span>
																			This {u.kind} references the resource by a hardcoded path
																			<span class="break-all font-mono">$res:{u.path}</span>.
																		</span>
																		<span class="text-hint">
																			For portability, prefer taking the resource as an input — a
																			fork won't have this exact path. It's relocated into the
																			project on publish, but converting it to an input keeps the
																			item reusable.
																		</span>
																	</div>
																{/snippet}
															</Popover>
														{/if}
													</span>
													{#if itemUrl}
														<a
															href={itemUrl}
															target="_blank"
															rel="noopener"
															title="Open {u.kind} in new tab"
															class="shrink-0 text-hint hover:text-primary"
														>
															<ExternalLink size={12} />
														</a>
													{/if}
												</div>
											</div>
										{/if}
									{/each}
								</div>
							</div>
						{/each}
					{/if}
				</div>
			</DrawerContent>
		</Drawer>

		<Drawer bind:this={triggerDrawer} size="640px">
			<DrawerContent title="Triggers" on:close={() => triggerDrawer?.closeDrawer()}>
				<div class="flex flex-col gap-4">
					<p class="text-xs text-secondary">
						Triggers attached to the selected scripts and flows. Synced to the Hub as
						<span class="font-semibold">disabled stubs</span>. Recipients review and enable each one
						manually after importing. External hooks (Slack/Discord webhooks, message-queue
						subscriptions, etc.) must be re-registered against the importing instance.
					</p>
					{#if s.relevantTriggers.length === 0}
						<span class="text-xs text-hint">No triggers reference the selected items.</span>
					{:else}
						{#each s.triggersByKind as [kind, triggers] (kind)}
							<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
								<div class="flex items-center gap-2 border-b pb-2">
									<span
										class="rounded border bg-surface px-1.5 py-0.5 font-mono text-xs text-primary"
									>
										{TRIGGER_KINDS[kind].badge}
									</span>
									<span class="text-[11px] text-hint">
										{triggers.length} trigger{triggers.length > 1 ? 's' : ''}
									</span>
									{#if TRIGGER_KINDS[kind].note}
										<span class="ml-auto">
											<Popover notClickable placement="top">
												<Info size={12} class="text-blue-600 dark:text-blue-400" />
												{#snippet text()}
													<div
														class="flex w-72 max-w-[90vw] flex-col gap-1 text-left text-[11px] normal-case"
													>
														<span>{TRIGGER_KINDS[kind].note}</span>
													</div>
												{/snippet}
											</Popover>
										</span>
									{/if}
								</div>
								<div class="flex flex-col gap-3">
									{#each triggers as t (t.path)}
										{@const runnableSummary = s.runnableSummaryByPath.get(
											`${t.is_flow ? 'flow' : 'script'}:${t.script_path}`
										)}
										{@const details = triggerDetails(t)}
										{@const cfg = t.config as any}
										{@const previewKey =
											t.kind === 'schedule' ? `${cfg.schedule}|${cfg.timezone}` : ''}
										{@const preview =
											t.kind === 'schedule' ? s.schedulePreviews[previewKey] : undefined}
										{@const triggerUrl = s.triggerListUrl(t.kind)}
										<div class="flex flex-col gap-1 text-xs">
											<div class="flex items-center gap-2">
												{#if t.is_flow}
													<BarsStaggered size={14} class="shrink-0 text-hint" />
												{:else}
													<Code2 size={14} class="shrink-0 text-hint" />
												{/if}
												<span class="break-all font-mono text-primary">
													{runnableSummary || t.script_path}
												</span>
												<span
													class="ml-auto inline-flex shrink-0 items-center gap-1 rounded bg-surface px-1.5 py-px text-[10px] font-semibold uppercase tracking-wide text-secondary"
												>
													{t.is_flow ? 'flow' : 'script'}
												</span>
												{#if triggerUrl}
													<a
														href={triggerUrl}
														target="_blank"
														rel="noopener"
														title="Open {TRIGGER_KINDS[t.kind].badge} trigger list in new tab"
														class="shrink-0 text-hint hover:text-primary"
													>
														<ExternalLink size={12} />
													</a>
												{/if}
											</div>
											{#if details.length > 0}
												<dl class="ml-5 grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5 text-[11px]">
													{#each details as d (d.label)}
														<dt class="text-hint">{d.label}</dt>
														<dd class="break-all font-mono text-tertiary">{d.value}</dd>
													{/each}
													{#if t.kind === 'schedule'}
														<dt class="text-hint">Next runs</dt>
														<dd class="text-tertiary">
															{#if preview && preview.length > 0}
																<div class="flex flex-col gap-0.5">
																	{#each preview as date (date)}
																		<span>{displayDate(date)}</span>
																	{/each}
																</div>
															{:else if preview && preview.length === 0}
																<span class="text-hint">No upcoming run</span>
															{:else}
																<span class="text-hint">Loading…</span>
															{/if}
														</dd>
													{/if}
												</dl>
											{/if}
										</div>
									{/each}
								</div>
							</div>
						{/each}
					{/if}
				</div>
			</DrawerContent>
		</Drawer>

		<Drawer bind:this={bundleDrawer} size="600px">
			<DrawerContent title="Bundle to Hub" on:close={() => bundleDrawer?.closeDrawer()}>
				<div class="flex flex-col gap-4">
					<p class="text-xs text-secondary">
						Name and document your bundle. The readme can be updated later, but a clear one speeds
						up the Windmill team's review.
					</p>
					{#if s.bundlePreview && s.bundlePreview.unresolved.length > 0}
						<div
							class="flex flex-col gap-1 rounded-md border border-red-300 bg-red-50 p-3 text-xs text-red-800 dark:border-red-800 dark:bg-red-950/40 dark:text-red-200"
						>
							<span class="font-semibold"
								>{s.bundlePreview.unresolved.length} unresolved reference(s) — cannot publish</span
							>
							<span class="text-[11px]">
								These items or resources couldn't be resolved, so the bundle would ship broken
								references. Deselect or fix them, then retry:
							</span>
							<ul class="list-disc pl-4 font-mono text-[11px]">
								{#each s.bundlePreview.unresolved as u (u)}
									<li class="break-all">{u}</li>
								{/each}
							</ul>
						</div>
					{/if}
					<label class="flex flex-col gap-1 text-xs">
						<span class="font-semibold text-primary">Name</span>
						<TextInput
							bind:value={s.hubName}
							inputProps={{ placeholder: 'e.g. Acme CRM toolkit' }}
						/>
					</label>
					<div class="flex flex-col gap-1 text-xs">
						<span class="font-semibold text-primary">Project slug</span>
						<span class="rounded border bg-surface-secondary px-2 py-1.5 font-mono text-secondary">
							{s.effectiveSlug || sanitizeSlug(s.hubName) || '—'}
						</span>
						<span class="text-[11px] text-hint">
							{#if s.effectiveSlug}
								Locked — items live under <span class="font-mono">f/{s.effectiveSlug}/</span>.
							{:else if s.hubName.trim() && !isValidSlug(sanitizeSlug(s.hubName))}
								<span class="text-red-600 dark:text-red-400">
									The name yields an invalid slug. Use at least 3 letters/digits.
								</span>
							{:else}
								Auto-generated from the name. Once project forked, items will live under
								<span class="font-mono">f/{sanitizeSlug(s.hubName) || '<slug>'}/</span>.
							{/if}
						</span>
					</div>
					<label class="flex flex-col gap-1 text-xs">
						<span class="font-semibold text-primary">Summary</span>
						<TextInput
							bind:value={s.hubSummary}
							inputProps={{ placeholder: 'Short one-liner shown on the Hub card' }}
						/>
					</label>
					<label class="flex flex-col gap-1 text-xs">
						<span class="font-semibold text-primary">Readme</span>
						<textarea
							bind:value={s.hubReadme}
							placeholder={"# What this workspace does\n\n# Who it's for\n\n# How to use it\n"}
							rows="10"
							class="rounded border px-2 py-1.5 text-xs font-mono bg-surface"
						></textarea>
						<span class="text-[11px] text-hint">
							Markdown supported. Editable any time before and after publication.
						</span>
					</label>
					<div class="flex flex-col gap-2 border-t pt-4 text-xs">
						<div class="flex items-center gap-2">
							<Database size={14} />
							<span class="font-semibold text-primary">Data table migrations</span>
						</div>
						{#if s.migrationsGenerating}
							<div class="flex items-center gap-2 text-secondary">
								<Loader2 size={14} class="animate-spin" />
								Detecting data tables used by this project…
							</div>
						{:else if s.migrationDrafts.length === 0}
							<span class="text-[11px] text-hint">
								No data table usage detected in this project's scripts, flows, or raw apps.
							</span>
						{:else}
							<span class="text-[11px] text-hint">
								We detected these data tables. When included, the migration recreates their tables
								on import. Best-effort — review and edit before publishing.
							</span>
							{#each s.migrationDrafts as m (m.datatable_name)}
								<div class="flex flex-col gap-1.5 rounded border bg-surface-secondary p-2">
									<div class="flex items-center justify-between gap-2">
										<span class="font-mono text-primary">{m.datatable_name}</span>
										<Toggle bind:checked={m.enabled} size="xs" options={{ right: 'Include' }} />
									</div>
									<MigrationSqlEditor
										bind:up={m.sql}
										bind:down={m.sql_down}
										generation={s.migrationsGeneration}
									/>
								</div>
							{/each}
						{/if}
					</div>
				</div>
				{#snippet actions()}
					<Button
						variant="accent"
						loading={s.deploying}
						disabled={!s.hubName.trim() ||
							(!s.effectiveSlug && !isValidSlug(sanitizeSlug(s.hubName))) ||
							s.migrationsGenerating ||
							s.triggersLoading ||
							s.triggerDiscoveryFailed ||
							(s.bundlePreview?.unresolved.length ?? 0) > 0}
						startIcon={{ icon: Cloud }}
						onclick={confirmBundle}
					>
						Create bundle
					</Button>
				{/snippet}
			</DrawerContent>
		</Drawer>
	{/key}
{/if}
