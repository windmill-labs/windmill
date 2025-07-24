<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { Button, Alert, Badge } from '$lib/components/common'
	import { Loader2, CheckCircle2, XCircle, Terminal, ChevronDown, ChevronUp } from 'lucide-svelte'
	import GitDiffPreview from '../GitDiffPreview.svelte'
	import { JobService, WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import hubPaths from '$lib/hubPaths.json'
	import { tryEvery } from '$lib/utils'
	import type { SyncResponse, SettingsObject } from '$lib/git-sync'

	interface Props {
		open: boolean
		gitRepoResourcePath: string
		uiState: SettingsObject
		repoIndex?: number
		currentGitSyncSettings?: any
		onFilterUpdate?: (filters: SettingsObject) => void
		onSettingsSaved?: () => void
		onClose: () => void
		onSuccess?: () => void
	}

	let {
		open = $bindable(false),
		gitRepoResourcePath,
		uiState,
		repoIndex,
		currentGitSyncSettings,
		onFilterUpdate,
		onSettingsSaved,
		onClose,
		onSuccess
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
	let previewResult = $state<SyncResponse | null>(null)

	// Note: Escape key is handled by the Modal component itself

	// Auto-save settings after successful pull with settings updates
	async function saveUpdatedSettings() {
		if (!currentGitSyncSettings || repoIndex === undefined) return

		try {
			const workspace = $workspaceStore
			if (!workspace) return

			// Create a serialized version of repositories
			const repositories = currentGitSyncSettings.repositories.map((repo: any) => {
				const serialized: any = {
					script_path: repo.script_path,
					git_repo_resource_path: repo.git_repo_resource_path,
					use_individual_branch: repo.use_individual_branch || false,
					group_by_folder: repo.group_by_folder || false
				}

				// Add settings if they exist
				if (repo.settings) {
					serialized.settings = {
						include_path: repo.settings.include_path || [],
						exclude_path: repo.settings.exclude_path || [],
						extra_include_path: repo.settings.extra_include_path || [],
						include_type: repo.settings.include_type || []
					}
				}

				// Add exclude_types_override if it exists
				if (repo.exclude_types_override && repo.exclude_types_override.length > 0) {
					serialized.exclude_types_override = repo.exclude_types_override
				}

				return serialized
			})

			await WorkspaceService.editWorkspaceGitSyncConfig({
				workspace,
				requestBody: {
					git_sync_settings: { repositories }
				}
			})

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


	// Close modal handler
	function handleClose() {
		if (!isPreviewLoading && !isApplying) {
			open = false
			onClose()
		}
	}
</script>


<Modal bind:open title="Pull Workspace from Git Repository" class="sm:max-w-4xl">
	<div class="flex flex-col gap-4">
		<!-- Description -->
		<p class="text-sm text-secondary">Pull and apply changes from the Git repository to your workspace. If settings changes are detected, you can choose to pull just the settings or everything.</p>

		<!-- Warning about overwrites -->
		<Alert type="warning" title="This will overwrite local changes">
			Pulling from the repository will overwrite any local changes to files that exist in the repository.
			Make sure to preview the changes before applying.
		</Alert>

		<!-- Preview section -->
		{#if !previewResult}
			<div class="flex justify-start pt-4">
				<Button
					size="md"
					color="dark"
					onclick={() => executeJob(true)}
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
			<div class="space-y-4">
				<!-- File changes -->
				<div>
					<h4 class="text-sm font-semibold text-primary mb-2">Workspace changes to pull</h4>

					{#if previewResult.changes?.length > 0}
						<GitDiffPreview previewResult={previewResult} />
					{:else}
						<div class="bg-surface-secondary rounded-lg p-3">
							<div class="text-sm text-tertiary">No changes to pull from the repository.</div>
						</div>
					{/if}
				</div>

				<!-- Settings changes -->
				{#if previewResult.settingsDiffResult?.hasChanges}
					<div class="border-t pt-4">
						<h4 class="text-sm font-semibold text-primary mb-2">
							Filter Settings from Repository
							<Badge color="blue" size="xs" class="ml-2">wmill.yaml</Badge>
						</h4>

						<div class="bg-surface-secondary rounded-lg p-4 space-y-1">
							{#if previewResult.settingsDiffResult.diff}
								{#each Object.entries(previewResult.settingsDiffResult.diff) as [field, change]}
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
			</div>
		{/if}

		<!-- Apply section (shown after successful preview) -->
		{#if previewResult && !previewError}
			{@const hasSettingsChanges = previewResult.settingsDiffResult?.hasChanges}
			{@const hasFileChanges = previewResult.changes?.length > 0}
			{#if hasSettingsChanges || hasFileChanges}
			<div class="border-t pt-4 mt-4">
				<div class="flex justify-start gap-2">

					{#if hasSettingsChanges && hasFileChanges}
						<!-- Show both buttons when both settings and file changes are detected -->
						<div class="flex flex-col gap-2">
							<div class="text-xs text-tertiary">Settings changes detected - choose your pull option:</div>
							<div class="flex gap-2">
								<Button
									size="md"
									color="dark"
									variant="border"
									onclick={() => executeJob(false, true)}
									disabled={isApplying}
									startIcon={{
										icon: isApplying ? Loader2 : undefined,
										classes: isApplying ? 'animate-spin' : ''
									}}
								>
									{isApplying ? 'Pulling...' : 'Settings only'}
								</Button>
								<Button
									size="md"
									color="dark"
									variant="border"
									onclick={() => executeJob(false, false)}
									disabled={isApplying}
									startIcon={{
										icon: isApplying ? Loader2 : undefined,
										classes: isApplying ? 'animate-spin' : ''
									}}
								>
									{isApplying ? 'Pulling...' : 'Everything'}
								</Button>
								<Button
									size="md"
									color="light"
									onclick={handleClose}
									disabled={isApplying}
								>
									Cancel
								</Button>
							</div>
						</div>
					{:else if hasSettingsChanges}
						<!-- Show only settings button when only settings changes are detected -->
						<div class="flex gap-2">
							<Button
								size="md"
								color="primary"
								onclick={() => executeJob(false, true)}
								disabled={isApplying}
								startIcon={{
									icon: isApplying ? Loader2 : undefined,
									classes: isApplying ? 'animate-spin' : ''
								}}
							>
								{isApplying ? 'Pulling...' : 'Settings only'}
							</Button>
							<Button
								size="md"
								color="light"
								onclick={handleClose}
								disabled={isApplying}
							>
								Cancel
							</Button>
						</div>
					{:else if hasFileChanges}
						<!-- Show single pull button when only file changes (no settings changes) -->
						<div class="flex gap-2">
							<Button
								size="md"
								color="primary"
								onclick={() => executeJob(false, false)}
								disabled={isApplying}
								startIcon={{
									icon: isApplying ? Loader2 : undefined,
									classes: isApplying ? 'animate-spin' : ''
								}}
							>
								{isApplying ? 'Pulling...' : 'Pull from repository'}
							</Button>
							<Button
								size="md"
								color="light"
								onclick={handleClose}
								disabled={isApplying}
							>
								Cancel
							</Button>
						</div>
					{:else}
						<!-- No changes to pull - show only cancel -->
						<div class="flex gap-2">
							<Button
								size="md"
								color="light"
								onclick={handleClose}
							>
								Close
							</Button>
						</div>
					{/if}
				</div>
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

# Push from git repository to workspace
wmill sync push --workspace {$workspaceStore} --repository {gitRepoResourcePath}

# Push settings only from git repository
wmill gitsync-settings push --workspace {$workspaceStore} --repository {gitRepoResourcePath}</pre>
				</div>
			{/if}
		</div>
	</div>
</Modal>
