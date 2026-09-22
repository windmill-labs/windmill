<!--
@component
Lists what pushing the workspace would commit, and pushes it. The host owns the
buttons — the setup dialog puts them in its own footer, the standalone modal in its
— and drives them through `apply()` and `status()`.
-->
<script lang="ts">
	import { untrack } from 'svelte'
	import { Alert, CopyButton } from '$lib/components/common'
	import { Loader2, XCircle, Terminal, ChevronDown, ChevronUp } from 'lucide-svelte'
	import GitDiffPreview from '../GitDiffPreview.svelte'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import hubPaths from '$lib/hubPaths.json'
	import { jobManager } from '$lib/services/JobManager'
	import type { SyncResponse, SettingsObject } from '$lib/git-sync'

	interface Props {
		gitRepoResourcePath: string
		uiState: SettingsObject
		onSuccess?: () => void
	}

	let { gitRepoResourcePath, uiState, onSuccess }: Props = $props()

	let previewJobId = $state<string | null>(null)
	let previewJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let isPreviewLoading = $state(false)
	let previewError = $state('')

	let applyJobId = $state<string | null>(null)
	let applyJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let isApplying = $state(false)
	let applyError = $state('')

	let showCliInstructions = $state(false)
	const cliInstructions = $derived(`# Setup (only needed if local folder not initialized yet)
npm install -g windmill-cli
wmill workspace add ${$workspaceStore} ${$workspaceStore} ${window.location.origin}
wmill init --workspace ${$workspaceStore} --repository ${gitRepoResourcePath}

# Pull workspace content to git repository
wmill sync pull --workspace ${$workspaceStore} --repository ${gitRepoResourcePath}`)
	let previewResult = $state<SyncResponse | null>(null)

	/** What the host needs to label and enable its buttons. */
	export function status() {
		return {
			previewing: isPreviewLoading,
			applying: isApplying,
			// Nothing to push is not worth a push job.
			canApply:
				!!previewResult && !previewError && !isApplying && (previewResult.changes?.length ?? 0) > 0
		}
	}

	export function apply() {
		return executeJob(false)
	}

	// The list of changes is what this is for, so it loads on its own rather than on a button.
	$effect(() => {
		gitRepoResourcePath
		untrack(() => void executeJob(true))
	})

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

			const result = await jobManager.runWithProgress(() => Promise.resolve(jobId), {
				workspace,
				timeout: 60000,
				timeoutMessage: `${isPreview ? 'Preview' : 'Apply'} job timed out after 60s`,
				onProgress: (status) => {
					if (isPreview) {
						previewJobStatus = status.status
					} else {
						applyJobStatus = status.status
					}

					if (status.status === 'failure') {
						if (isPreview) {
							previewError = status.error || 'Listing the changes failed'
						} else {
							applyError = status.error || 'Push failed'
						}
					}
				}
			})

			if (isPreview) {
				if (previewJobStatus === 'success') {
					previewResult = result as SyncResponse
				}
			} else {
				if (applyJobStatus === 'success') {
					onSuccess?.()
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
</script>

<div class="flex flex-col gap-4 h-full">
	{#if isPreviewLoading}
		<div class="flex-1 min-h-24 flex flex-col items-center justify-center gap-2 text-sm text-hint">
			<Loader2 size={36} class="animate-spin" />
			Listing the changes to push...
		</div>
	{:else}
		<p class="text-xs text-secondary">
			Here is what will get pushed to the repository, according to the filters you set.
		</p>
	{/if}

	{#if previewError}
		<Alert type="error" title="Could not list the changes">
			{previewError}
		</Alert>
	{/if}

	{#if previewResult && !previewError}
		<div class="flex flex-col gap-2">
			<h4 class="text-sm font-semibold text-primary">Changes to push</h4>

			{#if previewResult.changes?.length > 0}
				<GitDiffPreview {previewResult} maxHeightClass="max-h-80" />
			{:else}
				<div class="text-sm text-secondary">
					The repository already matches this workspace: there is nothing to push.
				</div>
			{/if}
		</div>
	{/if}

	{#if applyError}
		<Alert type="error" title="Push failed">
			{applyError}
		</Alert>
	{/if}

	<div class="mt-auto flex flex-col gap-2">
		{#each [{ id: previewJobId, status: previewJobStatus, label: 'Changes job', running: isPreviewLoading }, { id: applyJobId, status: applyJobStatus, label: 'Push job', running: isApplying }] as job (job.label)}
			<!-- The row is held from the moment the job is started, since its id only arrives once
			     the server has taken it; it goes once the job succeeded, its outcome being on
			     screen already. -->
			{#if job.running || (job.id && job.status !== 'success')}
				<div class="flex items-center gap-2 min-h-4 text-2xs text-secondary">
					{#if job.status === 'running'}
						<Loader2 class="animate-spin" size={12} />
					{:else if job.status === 'failure'}
						<XCircle size={12} class="text-red-700" />
					{/if}
					{#if job.id}
						<span class="text-hint">{job.label}:</span>
						<a
							target="_blank"
							class="underline"
							href={`/run/${job.id}?workspace=${$workspaceStore}`}
						>
							{job.id}
						</a>
					{/if}
				</div>
			{/if}
		{/each}

		<!-- Nothing to copy commands for until the changes are known. -->
		<div class={isPreviewLoading ? 'hidden' : ''}>
			<button
				class="flex items-center gap-2 text-xs text-secondary hover:text-primary transition-colors"
				onclick={() => (showCliInstructions = !showCliInstructions)}
			>
				<Terminal size={14} />
				<span>CLI instructions</span>
				{#if showCliInstructions}
					<ChevronUp size={14} />
				{:else}
					<ChevronDown size={14} />
				{/if}
			</button>

			{#if showCliInstructions}
				<div class="relative mt-2">
					<CopyButton value={cliInstructions} class="absolute top-1 right-1" />
					<pre
						class="text-xs bg-surface-sunken rounded-lg p-3 pr-10 overflow-x-auto whitespace-pre-wrap break-all"
						>{cliInstructions}</pre
					>
				</div>
			{/if}
		</div>
	</div>
</div>
