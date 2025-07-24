<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { Button, Alert } from '$lib/components/common'
	import { Loader2, CheckCircle2, XCircle, Terminal, ChevronDown, ChevronUp, Save } from 'lucide-svelte'
	import GitDiffPreview from '../GitDiffPreview.svelte'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import hubPaths from '$lib/hubPaths.json'
	import { tryEvery } from '$lib/utils'
	import type { SyncResponse, SettingsObject } from '$lib/git-sync'

	interface Props {
		open: boolean
		gitRepoResourcePath: string
		uiState: SettingsObject
		onClose: () => void
		onSuccess?: () => void
	}

	let {
		open = $bindable(false),
		gitRepoResourcePath,
		uiState,
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
	async function executeJob(isDryRun: boolean) {
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
				pull: false,
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
					onSuccess?.()
				} else {
					applyError = 'Push failed'
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


<Modal bind:open title="Push Workspace to Git Repository" class="sm:max-w-4xl">
	<div class="flex flex-col gap-4">
		<!-- Description -->
		<p class="text-sm text-secondary">Push your current workspace content to the connected Git repository based on the configured filters.</p>

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
			<div class="space-y-3">
				<h4 class="text-sm font-semibold text-primary">Changes to Push</h4>

				{#if previewResult.changes?.length > 0}
					<GitDiffPreview previewResult={previewResult} />
				{:else}
					<div class="bg-surface-secondary rounded-lg p-3">
						<div class="text-sm text-tertiary">No changes to push to the repository.</div>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Apply section (shown after successful preview) -->
		{#if previewResult && !previewError}
			{@const hasChanges = previewResult.changes?.length > 0}
			{#if hasChanges}
			<div class="border-t pt-4 mt-4">
				<div class="flex justify-start gap-2">
					<Button
						size="xs"
						onclick={() => executeJob(false)}
						disabled={isApplying}
						startIcon={{
							icon: isApplying ? Loader2 : undefined,
							classes: isApplying ? 'animate-spin' : ''
						}}
					>
						{isApplying ? 'Pushing...' : 'Push to repository'}
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
				Push job:
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
			<Alert type="error" title="Push failed">
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

# Pull workspace content to git repository
wmill sync pull --workspace {$workspaceStore} --repository {gitRepoResourcePath}</pre>
				</div>
			{/if}
		</div>
	</div>
</Modal>
