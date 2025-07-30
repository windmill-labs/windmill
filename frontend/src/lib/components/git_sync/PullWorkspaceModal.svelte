<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { Button, Alert, Badge } from '$lib/components/common'
	import { Loader2, CheckCircle2, XCircle, Terminal, ChevronDown, ChevronUp, Save } from 'lucide-svelte'
	import GitDiffPreview from '../GitDiffPreview.svelte'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import hubPaths from '$lib/hubPaths.json'
	import { tryEvery } from '$lib/utils'
	import type { SyncResponse, SettingsResponse, SettingsObject } from '$lib/git-sync'

	interface Props {
		open: boolean
		gitRepoResourcePath: string
		uiState: SettingsObject
		repoIndex?: number
		currentGitSyncSettings?: any
		onFilterUpdate?: (filters: SettingsObject) => void
		onSettingsSaved?: () => void
		onSuccess?: () => void
		settingsOnly?: boolean
	}

	let {
		open = $bindable(false),
		gitRepoResourcePath,
		uiState,
		repoIndex,
		currentGitSyncSettings,
		onFilterUpdate,
		onSettingsSaved,
		onSuccess,
		settingsOnly = false
	}: Props = $props()

	// Job state
	let previewJobId = $state<string | null>(null)
	let previewJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let isPreviewLoading = $state(false)
	let previewError = $state('')

	let applyJobId = $state<string | null>(null)
	let applyJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let isApplying = $state(false)
	let applyError = $state('')

	// UI state
	let showCliInstructions = $state(false)
	let previewResult = $state<SyncResponse | SettingsResponse | null>(null)
	let settingsApplied = $state(false)

	// Helper functions to reduce type casting repetition
	const getSettingsChanges = (result: SyncResponse | SettingsResponse | null) => {
		if (!result) return { hasChanges: false, data: null, diff: null }

		if (settingsOnly) {
			const settingsResponse = result as SettingsResponse
			return {
				hasChanges: settingsResponse.hasChanges ?? false,
				data: settingsResponse.local,
				diff: settingsResponse
			}
		} else {
			const syncResponse = result as SyncResponse
			return {
				hasChanges: syncResponse.settingsDiffResult?.hasChanges ?? false,
				data: syncResponse.settingsDiffResult?.local,
				diff: syncResponse.settingsDiffResult
			}
		}
	}

	const getWorkspaceChanges = (result: SyncResponse | SettingsResponse | null) => {
		if (!result || settingsOnly) return { hasChanges: false, changes: [] }
		const syncResponse = result as SyncResponse
		return {
			hasChanges: (syncResponse.changes?.length ?? 0) > 0,
			changes: syncResponse.changes ?? []
		}
	}

	// Auto-save settings after successful pull with settings updates
	async function saveUpdatedSettings() {
		if (!currentGitSyncSettings || repoIndex === undefined) return

		try {
			// Save only the specific repository that was updated
			await currentGitSyncSettings.saveRepository(repoIndex)
			onSettingsSaved?.()
		} catch (error) {
			console.error('Failed to save settings:', error)
			sendUserToast('Failed to save updated settings', true)
		}
	}

	// Reset state when modal opens/closes
	$effect(() => {
		if (!open) {
			previewJobId = null
			previewJobStatus = undefined
			isPreviewLoading = false
			previewError = ''
			applyJobId = null
			applyJobStatus = undefined
			isApplying = false
			applyError = ''
			showCliInstructions = false
			previewResult = null
			settingsApplied = false
		} else if (settingsOnly && !previewResult && !isPreviewLoading) {
			// Auto-trigger settings preview when modal opens in settings-only mode
			executeJob(true, true)
		}
	})

	// Execute job with dry run or actual execution
	async function executeJob(isDryRun: boolean, settingsOnly: boolean = false) {
		const isPreview = isDryRun

		if (isPreview) {
			isPreviewLoading = true
			previewError = ''
			previewResult = null
			previewJobId = null
			previewJobStatus = undefined
		} else {
			isApplying = true
			applyError = ''
			applyJobId = null
			applyJobStatus = undefined
		}

		try {
			const workspace = $workspaceStore
			if (!workspace) return

			const payload = {
				workspace_id: workspace,
				repo_url_resource_path: gitRepoResourcePath,
				dry_run: isDryRun,
				pull: true,
				only_wmill_yaml: settingsOnly,
				settings_json: JSON.stringify(uiState)
			}

			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: payload,
				skipPreprocessor: true
			})

			if (isPreview) {
				previewJobId = jobId
				previewJobStatus = 'running'
			} else {
				applyJobId = jobId
				applyJobStatus = 'running'
			}

			let jobSuccess = false
			let result: any = {}

			await tryEvery({
				tryCode: async () => {
					const testResult = await JobService.getCompletedJob({ workspace, id: jobId })
					jobSuccess = !!testResult.success
					if (jobSuccess) {
						const jobResult = await JobService.getCompletedJobResult({ workspace, id: jobId })
						result = jobResult
					}
				},
				timeoutCode: async () => {
					try {
						await JobService.cancelQueuedJob({
							workspace,
							id: jobId,
							requestBody: { reason: `${isPreview ? 'Preview' : 'Apply'} job timed out after 60s` }
						})
					} catch (err) {}
				},
				interval: 500,
				timeout: 60000
			})

			if (isPreview) {
				previewJobStatus = jobSuccess ? 'success' : 'failure'
				if (jobSuccess) {
					previewResult = result
				} else {
					previewError = 'Preview failed'
				}
			} else {
				applyJobStatus = jobSuccess ? 'success' : 'failure'
				if (jobSuccess) {
					const settingsData = result?.local
					const hasSettingsChanges = settingsData && onFilterUpdate
					if (hasSettingsChanges) {
						onFilterUpdate(settingsData)
						await saveUpdatedSettings()
					}
					onSuccess?.()
				} else {
					applyError = 'Pull failed'
				}
			}
		} catch (e) {
			const errorMsg = e?.message || 'Operation failed'
			if (isPreview) {
				previewJobStatus = 'failure'
				previewError = errorMsg
			} else {
				applyJobStatus = 'failure'
				applyError = errorMsg
			}
		} finally {
			if (isPreview) {
				isPreviewLoading = false
			} else {
				isApplying = false
			}
		}
	}


	// Apply settings only (no job needed - we have the data from preview)
	async function applySettingsOnly() {
		isApplying = true

		try {
			const settingsChanges = getSettingsChanges(previewResult)

			if (!settingsChanges.hasChanges) {
				sendUserToast('No settings changes to apply', true)
				return
			}

			if (!settingsChanges.data) {
				sendUserToast('Settings data not available', true)
				return
			}

			// Update the UI state with the new settings
			if (onFilterUpdate) {
				onFilterUpdate(settingsChanges.data)
			}

			// Save the updated settings
			await saveUpdatedSettings()

			if (settingsOnly) {
				// Settings-only mode - we're done, onSuccess will handle the toast
				onSuccess?.()
			} else {
				// Two-step flow - transition to step 2
				settingsApplied = true
				sendUserToast('Settings applied successfully. You can now review workspace changes.')
			}

		} catch (error: any) {
			console.error('Failed to apply settings:', error)
			sendUserToast('Failed to apply settings: ' + error.message, true)
		} finally {
			isApplying = false
		}
	}


</script>


<Modal bind:open title={settingsOnly ? "Pull Settings from Git Repository" : "Pull Workspace from Git Repository"} class="sm:max-w-4xl" cancelText={settingsOnly && !getSettingsChanges(previewResult).hasChanges ? "Close" : "Cancel"}>
	<div class="flex flex-col gap-4">
		<!-- Description -->
		<p class="text-sm text-secondary">
			{#if settingsOnly}
				Pull and apply settings changes from the Git repository to your workspace. This will update your sync filter settings only.
			{:else}
				Pull and apply changes from the Git repository to your workspace. If settings changes are detected, you can choose to pull just the settings or everything.
			{/if}
		</p>

		<!-- Warning about overwrites - only show for full pulls, not settings-only -->
		{#if !settingsOnly}
			<Alert type="warning" title="This will overwrite local changes">
				Pulling from the repository will overwrite any local changes to files that exist in the repository.
				Make sure to preview the changes before applying.
			</Alert>
		{/if}

		<!-- Preview section -->
		{#if !previewResult}
			<div class="flex justify-start pt-4">
				<Button
					size="md"
					color="dark"
					onclick={() => executeJob(true, settingsOnly)}
					disabled={isPreviewLoading}
					startIcon={{
						icon: isPreviewLoading ? Loader2 : undefined,
						classes: isPreviewLoading ? 'animate-spin' : ''
					}}
				>
					{isPreviewLoading ? 'Previewing...' : 'Preview changes'}
				</Button>
			</div>
		{/if}

		<!-- Job status for preview -->
		{#if previewJobId}
			<div class="flex items-center gap-2 text-xs text-tertiary">
				{#if previewJobStatus === 'running'}
					<Loader2 class="animate-spin" size={14} />
				{:else if previewJobStatus === 'success'}
					<CheckCircle2 size={14} class="text-green-600" />
				{:else if previewJobStatus === 'failure'}
					<XCircle size={14} class="text-red-700" />
				{/if}
				Preview job:
				<a
					target="_blank"
					class="underline"
					href={`/run/${previewJobId}?workspace=${$workspaceStore}`}
				>
					{previewJobId}
				</a>
			</div>
		{/if}

		<!-- Preview error -->
		{#if previewError}
			<Alert type="error" title="Preview failed">
				{previewError}
			</Alert>
		{/if}

		<!-- Preview results -->
		{#if previewResult && !previewError}
			{@const settingsChanges = getSettingsChanges(previewResult)}
			{@const workspaceChanges = getWorkspaceChanges(previewResult)}
			<div class="space-y-4">
				<!-- Settings changes (always show first if present) -->
				{#if settingsChanges.hasChanges && !settingsApplied}
					<div>
						<h4 class="text-sm font-semibold text-primary mb-2">
							Filter Settings from Repository
							<Badge color="blue" size="xs" class="ml-2">wmill.yaml</Badge>
						</h4>

						<div class="bg-surface-secondary rounded-lg p-4 space-y-1">
							{#if settingsChanges.diff?.diff}
								{#each Object.entries(settingsChanges.diff.diff) as [field, change]}
									{@const fieldName = field.replace(/([A-Z])/g, ' $1').replace(/^./, str => str.toUpperCase())}
									{@const typedChange = change as {from: any, to: any}}
									<div class="flex items-center gap-2 text-xs">
										<span class="text-tertiary min-w-0 flex-shrink-0">{fieldName}:</span>
										{#if Array.isArray(typedChange.from) && Array.isArray(typedChange.to)}
											<span class="text-red-600">{typedChange.from.length === 0 ? 'None' : typedChange.from.join(', ')}</span>
											<span class="text-tertiary">→</span>
											<span class="text-green-600">{typedChange.to.length === 0 ? 'None' : typedChange.to.join(', ')}</span>
										{:else}
											<span class="text-red-600">{typedChange.from}</span>
											<span class="text-tertiary">→</span>
											<span class="text-green-600">{typedChange.to}</span>
										{/if}
									</div>
								{/each}
							{:else}
								<div class="text-xs text-tertiary">
									Settings changes detected but no detailed diff available.
								</div>
							{/if}
						</div>
					</div>
				{/if}

				<!-- No settings changes detected (settings-only mode) -->
				{#if settingsOnly && !settingsChanges.hasChanges}
					<div class="bg-surface-secondary rounded-lg p-3">
						<div class="text-sm text-tertiary">
							No settings changes detected. Your local sync filter settings are already up to date with the repository.
						</div>
					</div>
				{/if}

				<!-- Workspace changes (show when no settings changes and there are workspace changes) -->
				{#if !settingsOnly && !settingsChanges.hasChanges && workspaceChanges.hasChanges}
					<div class={settingsChanges.hasChanges && settingsApplied ? 'border-t pt-4' : ''}>
						<h4 class="text-sm font-semibold text-primary mb-2">Workspace changes to pull</h4>

						{#if workspaceChanges.hasChanges}
							<GitDiffPreview previewResult={previewResult as SyncResponse} />
						{:else}
							<div class="bg-surface-secondary rounded-lg p-3">
								<div class="text-sm text-tertiary">No changes to pull from the repository.</div>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/if}

		<!-- Apply section (shown after successful preview) -->
		{#if previewResult && !previewError}
			{@const settingsChanges = getSettingsChanges(previewResult)}
			{@const workspaceChanges = getWorkspaceChanges(previewResult)}

			{#if settingsChanges.hasChanges || workspaceChanges.hasChanges}
				<div class="border-t pt-4 mt-4">
					{#if settingsChanges.hasChanges && workspaceChanges.hasChanges && !settingsApplied}
						<!-- Step 1: Settings changes first when both are present -->
						<div class="flex flex-col gap-3">
							<div class="text-sm font-medium text-primary">Step 1 of 2: Apply settings changes</div>
							<div class="text-xs text-tertiary">Settings changes detected. Apply these first to ensure workspace content is pulled with the correct configuration.</div>
							<div class="flex gap-2">
								<Button
									size="md"
									onclick={applySettingsOnly}
									disabled={isApplying}
									startIcon={{
										icon: isApplying ? Loader2 : Save,
										classes: isApplying ? 'animate-spin' : ''
									}}
								>
									{isApplying ? 'Applying...' : 'Apply settings'}
								</Button>
							</div>
						</div>
					{:else if settingsChanges.hasChanges && !workspaceChanges.hasChanges && !settingsApplied}
						<!-- Only settings changes -->
						<div class="flex gap-2">
							<Button
								size="md"
								onclick={applySettingsOnly}
								disabled={isApplying}
								startIcon={{
									icon: isApplying ? Loader2 : Save,
									classes: isApplying ? 'animate-spin' : ''
								}}
							>
								{isApplying ? 'Applying...' : 'Apply settings'}
							</Button>
						</div>
					{:else if workspaceChanges.hasChanges && (!settingsChanges.hasChanges || settingsApplied)}
						<!-- Step 2: Workspace changes (either no settings changes, or settings already applied) -->
						<div class="flex flex-col gap-3">
							{#if settingsApplied}
								<div class="text-sm font-medium text-primary">Step 2 of 2: Pull Workspace Changes</div>
								<div class="text-xs text-green-600">✓ Settings applied successfully. Now you can pull the workspace changes.</div>
							{/if}
							<div class="flex gap-2">
								<Button
									size="md"
									onclick={() => executeJob(false, false)}
									disabled={isApplying}
									startIcon={{
										icon: isApplying ? Loader2 : Save,
										classes: isApplying ? 'animate-spin' : ''
									}}
								>
									{isApplying ? 'Pulling...' : 'Pull from repository'}
								</Button>
							</div>
						</div>
					{:else}
						<!-- No changes to pull -->
						<div class="bg-surface-secondary rounded-lg p-3">
							<div class="text-sm text-tertiary">No changes to pull from the repository.</div>
						</div>
					{/if}
				</div>
			{/if}
		{/if}

		<!-- Job status for apply -->
		{#if applyJobId}
			<div class="flex items-center gap-2 text-xs text-tertiary">
				{#if applyJobStatus === 'running'}
					<Loader2 class="animate-spin" size={14} />
				{:else if applyJobStatus === 'success'}
					<CheckCircle2 size={14} class="text-green-600" />
				{:else if applyJobStatus === 'failure'}
					<XCircle size={14} class="text-red-700" />
				{/if}
				Pull job:
				<a
					target="_blank"
					class="underline"
					href={`/run/${applyJobId}?workspace=${$workspaceStore}`}
				>
					{applyJobId}
				</a>
			</div>
		{/if}

		<!-- Apply error -->
		{#if applyError}
			<Alert type="error" title="Pull failed">
				{applyError}
			</Alert>
		{/if}

		<!-- CLI Instructions (collapsible) -->
		<div class="border-t pt-4 mt-4">
			<button
				class="flex items-center gap-2 text-sm text-secondary hover:text-primary transition-colors"
				onclick={() => showCliInstructions = !showCliInstructions}
			>
				<Terminal size={16} />
				<span>CLI Instructions</span>
				{#if showCliInstructions}
					<ChevronUp size={16} />
				{:else}
					<ChevronDown size={16} />
				{/if}
			</button>

			{#if showCliInstructions}
				<div class="mt-3 bg-surface-secondary rounded-lg p-3">
					<pre class="text-xs bg-surface p-3 rounded overflow-x-auto whitespace-pre-wrap break-all">
# Setup (only needed if local folder not initialized yet)
npm install -g windmill-cli
wmill workspace add {$workspaceStore} {$workspaceStore} {window.location.origin}
wmill init --workspace {$workspaceStore} --repository {gitRepoResourcePath}

{#if !settingsOnly}
# Push from git repository to workspace
wmill sync push --workspace {$workspaceStore} --repository {gitRepoResourcePath}

{/if}
# Push settings only from git repository
wmill gitsync-settings push --workspace {$workspaceStore} --repository {gitRepoResourcePath}</pre>
				</div>
			{/if}
		</div>
	</div>
</Modal>
