<script lang="ts">
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import {
		Loader2,
		Eye,
		Save,
		CheckCircle2,
		XCircle,
		DownloadCloud,
		AlertTriangle,
		Terminal,
		ChevronDown,
		ChevronUp
	} from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import hubPaths from '$lib/hubPaths.json'
	import { JobService } from '$lib/gen'
	import { tryEvery } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import GitDiffPreview from './GitDiffPreview.svelte'
	import { page } from '$app/stores'

	let { gitRepoResourcePath, uiState, onFilterUpdate } = $props<{
		gitRepoResourcePath: string
		uiState: {
			include_path: string[]
			exclude_path: string[]
			extra_include_path: string[]
			include_type: string[]
		}
		onFilterUpdate: (filters: {
			include_path: string[]
			exclude_path: string[]
			extra_include_path: string[]
			include_type: string[]
		}) => void
	}>()

	type PreviewResult = {
		added: string[]
		deleted: string[]
		modified: string[]
	}

	let previewResult = $state<PreviewResult | undefined>(undefined)

	let isPreviewLoading = $state(false)
	let isPulling = $state(false)
	let pullGitRepoPopover = $state<{ open: () => void; close: () => void } | null>(null)
	let jobStatus = $state<{
		id: string | null
		status: 'running' | 'success' | 'failure' | undefined
		error?: string
		type: 'preview' | 'pull'
	}>({
		id: null,
		status: undefined,
		type: 'preview'
	})

	let isCliInfoExpanded = $state(false)

	async function handleJobCompletion(jobId: string, workspace: string): Promise<boolean> {
		let success = false
		await tryEvery({
			tryCode: async () => {
				const result = await JobService.getCompletedJob({
					workspace,
					id: jobId
				})
				success = !!result.success
			},
			timeoutCode: async () => {
				try {
					await JobService.cancelQueuedJob({
						workspace,
						id: jobId,
						requestBody: {
							reason: 'Job timed out after 5s'
						}
					})
				} catch (err) {
					console.error(err)
				}
			},
			interval: 500,
			timeout: 10000
		})
		return success
	}

	async function previewChanges() {
		const workspace = $workspaceStore
		if (!workspace) return

		console.log('Previewing changes for repo:', gitRepoResourcePath)
		isPreviewLoading = true
		jobStatus = { id: null, status: undefined, type: 'preview' }

		try {
			// Always use the simplified JSON approach
			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: {
					workspace_id: workspace,
					repo_url_resource_path: gitRepoResourcePath,
					dry_run: true,
					pull: true,
					only_wmill_yaml: false,
					settings_json: JSON.stringify(uiState)
				},
				skipPreprocessor: true
			})

			jobStatus = { id: jobId, status: 'running', type: 'preview' }
			const success = await handleJobCompletion(jobId, workspace)

			if (success) {
				const rawResult = await JobService.getCompletedJobResult({ workspace, id: jobId })
				console.log('Preview result:', rawResult)

				// Convert new CLI format to expected format
				const added: string[] = []
				const deleted: string[] = []
				const modified: string[] = []

				if (rawResult && rawResult.changes && Array.isArray(rawResult.changes)) {
					for (const change of rawResult.changes) {
						if (change.type === 'added') {
							added.push(change.path)
						} else if (change.type === 'deleted') {
							deleted.push(change.path)
						} else if (change.type === 'modified') {
							modified.push(change.path)
						}
					}
				}

				// For full sync mode, just use the CLI results directly
				// The CLI already handles wmill.yaml changes with --include-wmill-yaml flag
				previewResult = { added, deleted, modified }
				jobStatus.status = 'success'
			} else {
				previewResult = undefined
				jobStatus.status = 'failure'
			}
		} catch (error) {
			console.error('Failed to preview changes:', error)
			previewResult = undefined
			jobStatus = {
				...jobStatus,
				status: 'failure',
				error: error instanceof Error ? error.message : String(error)
			}
		} finally {
			isPreviewLoading = false
		}
	}

	async function pullFromRepo() {
		const workspace = $workspaceStore
		if (!workspace) return

		console.log('Pulling from repo:', gitRepoResourcePath)
		isPulling = true
		jobStatus = { id: null, status: undefined, type: 'pull' }

		try {
			// Use init git repo script with dry_run: false (actual pull operation)
			// The script will read wmill.yaml directly from the cloned repo, no need to pass settings
			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: {
					workspace_id: workspace,
					repo_url_resource_path: gitRepoResourcePath,
					dry_run: false,
					branch_to_push: '',
					only_wmill_yaml: false,
					pull: true,
					settings_json: undefined // Let script use wmill.yaml from repo
				},
				skipPreprocessor: true
			})

			jobStatus = { id: jobId, status: 'running', type: 'pull' }
			const success = await handleJobCompletion(jobId, workspace)
			jobStatus.status = success ? 'success' : 'failure'

			if (success) {
				// Get the result which should contain the local git repo settings as JSON
				const result = (await JobService.getCompletedJobResult({ workspace, id: jobId })) as any
				console.log('Pull result:', result)

				// Apply the settings from the sync operation result to the UI
				if (result?.settings_json) {
					// Directly update the UI state with the JSON result - no YAML conversion needed!
					const settingsJson = result.settings_json as {
						include_path: string[]
						exclude_path?: string[]
						extra_include_path?: string[]
						include_type: string[]
					}
					onFilterUpdate({
						include_path: settingsJson.include_path || ['f/**'],
						exclude_path: settingsJson.exclude_path || [],
						extra_include_path: settingsJson.extra_include_path || [],
						include_type: settingsJson.include_type || ['script', 'flow', 'app', 'folder']
					})
					sendUserToast('Successfully pulled workspace content from repository')

					// Reset popover state after successful pull
					previewResult = undefined
					jobStatus = { id: null, status: undefined, type: 'preview' }
					pullGitRepoPopover?.close()
				} else {
					console.warn('No settings_json returned from pull operation')
					sendUserToast('Pull completed but could not update filter settings', true)
				}
			}
		} catch (error) {
			console.error('Failed to pull from repo:', error)
			jobStatus = {
				...jobStatus,
				status: 'failure',
				error: error instanceof Error ? error.message : String(error)
			}
		} finally {
			isPulling = false
		}
	}
</script>

<Popover
	bind:this={pullGitRepoPopover}
	floatingConfig={{
		placement: 'top-start',
		strategy: 'fixed',
		flip: false,
		shift: true
	}}
	contentClasses="p-4 w-1/3"
>
	<svelte:fragment slot="trigger">
		<Button
			color="dark"
			size="sm"
			nonCaptureEvent
			onclick={pullGitRepoPopover?.open}
			startIcon={{ icon: DownloadCloud }}
		>
			Pull workspace from Git repo
		</Button>
	</svelte:fragment>

	<svelte:fragment slot="content" let:close>
		<div class="flex flex-col gap-4">
			<div class="flex flex-col gap-2">
				<h3 class="text-lg font-semibold">Pull workspace from Git repository</h3>
				<div class="prose max-w-none text-2xs text-tertiary">
					This action will pull all workspace objects from your Git repository according to the
					filters set in the Git repository wmill.yaml file and apply does filter settings to the
					workspace.
					<span class="text-orange-600 flex items-center gap-1">
						<AlertTriangle size={14} /> This will overwrite your current workspace content and Git sync
						filter settings with the content from the Git repository.
					</span>

					<!-- Collapsible CLI Info Section -->
					<div class="mt-2 border rounded-md">
						<button
							class="w-full flex items-center justify-between p-1.5 bg-surface-secondary hover:bg-surface-hover"
							onclick={() => (isCliInfoExpanded = !isCliInfoExpanded)}
						>
							<span class="font-medium flex items-center gap-2">
								<Terminal size={14} />
								Windmill CLI to push local files to Windmill
							</span>
							{#if isCliInfoExpanded}
								<ChevronUp size={16} />
							{:else}
								<ChevronDown size={16} />
							{/if}
						</button>

						{#if isCliInfoExpanded}
							<div class="p-1 bg-surface-tertiary">
								<div class="text-2xs mb-2">
									Not familiar with Windmill CLI? <a
										href="https://www.windmill.dev/docs/advanced/cli/sync"
										class="text-blue-500 hover:text-blue-600 underline"
										target="_blank"
										rel="noopener noreferrer">Check out the docs</a
									>
								</div>
								<div class="font-mono text-2xs">
									<pre class="overflow-auto max-h-60"
										><code
											>npm install -g windmill-cli
# Clone your git repository
git clone $REPO_URL
cd $REPO_NAME
# Configure Windmill CLI
wmill workspace add {$workspaceStore} {$workspaceStore} {`${$page.url.protocol}//${$page.url.hostname}/`}
# Push the content to Windmill
wmill sync push --yes
# Optional: add --skip-secrets --skip-variables --skip-resources flags as needed</code
										></pre
									>
								</div>
							</div>
						{/if}
					</div>
				</div>
			</div>

			{#if previewResult}
				<GitDiffPreview {previewResult} />
			{/if}

			{#if jobStatus.id}
				<div class="flex items-center gap-2 text-xs text-tertiary">
					{#if jobStatus.status === 'running'}
						<Loader2 class="animate-spin" size={14} />
					{:else if jobStatus.status === 'success'}
						<CheckCircle2 size={14} class="text-green-600" />
					{:else if jobStatus.status === 'failure'}
						<XCircle size={14} class="text-red-700" />
					{/if}
					{jobStatus.type === 'preview' ? 'Preview' : 'Pull'} job:
					<a
						target="_blank"
						class="underline"
						href={`/run/${jobStatus.id}?workspace=${$workspaceStore}`}
					>
						{jobStatus.id}
					</a>
				</div>
				{#if jobStatus.error}
					<div class="text-xs text-red-600">{jobStatus.error}</div>
				{/if}
			{/if}

			<div class="flex justify-between items-center mt-4">
				<Button
					color="light"
					size="xs"
					on:click={() => {
						previewResult = undefined
						jobStatus = { id: null, status: undefined, type: 'preview' }
						close()
					}}
					disabled={isPreviewLoading || isPulling}
				>
					Cancel
				</Button>
				<div class="flex gap-2">
					{#if !previewResult}
						<Button
							size="xs"
							on:click={previewChanges}
							disabled={isPreviewLoading || isPulling}
							startIcon={{
								icon: isPreviewLoading ? Loader2 : Eye,
								classes: isPreviewLoading ? 'animate-spin' : ''
							}}
						>
							Preview
						</Button>
					{:else}
						<Button
							size="xs"
							on:click={previewChanges}
							disabled={isPreviewLoading || isPulling}
							startIcon={{
								icon: isPreviewLoading ? Loader2 : Eye,
								classes: isPreviewLoading ? 'animate-spin' : ''
							}}
							title="Preview changes again"
						>
							Preview
						</Button>
						{#if previewResult.added?.length || previewResult.deleted?.length || previewResult.modified?.length}
							<Button
								color="red"
								size="xs"
								on:click={pullFromRepo}
								disabled={isPreviewLoading || isPulling}
								startIcon={{
									icon: isPulling ? Loader2 : Save,
									classes: isPulling ? 'animate-spin' : ''
								}}
							>
								{isPulling ? 'Pulling...' : 'Pull'}
							</Button>
						{/if}
					{/if}
				</div>
			</div>
		</div>
	</svelte:fragment>
</Popover>
