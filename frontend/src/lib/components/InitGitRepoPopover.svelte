<script lang="ts">
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Alert } from '$lib/components/common'
	import {
		Loader2,
		Eye,
		Save,
		CheckCircle2,
		XCircle,
		UploadCloud,
		AlertTriangle,
		Terminal,
		ChevronDown,
		ChevronUp
	} from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import hubPaths from '$lib/hubPaths.json'
	import { JobService } from '$lib/gen'
	import { tryEvery } from '$lib/utils'
	import GitDiffPreview from './GitDiffPreview.svelte'
	import { page } from '$app/stores'

	let { gitRepoResourcePath, branchName, uiState } = $props<{
		gitRepoResourcePath: string
		branchName?: string
		uiState: {
			include_path: string[]
			exclude_path: string[]
			extra_include_path: string[]
			include_type: string[]
		}
	}>()

	let _branchName = $state(branchName ?? '')
	let previewResult = $state<
		| {
				added: string[]
				deleted: string[]
				modified: string[]
		  }
		| undefined
	>(undefined)
	let isPreviewLoading = $state(false)
	let isInitializing = $state(false)
	let initResult = $state<{ success: boolean; message: string | undefined } | null>(null)
	let initGitRepoPopover = $state<{ open: () => void; close: () => void } | null>(null)
	let previewJobId = $state<string | null>(null)
	let previewJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let pushJobId = $state<string | null>(null)
	let pushJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let isCliInfoExpanded = $state(false)

	async function previewChanges() {
		console.log('Previewing changes for repo:', gitRepoResourcePath)
		isPreviewLoading = true
		previewJobId = null
		previewJobStatus = undefined
		try {
			const workspace = $workspaceStore
			if (!workspace) {
				previewResult = undefined
				isPreviewLoading = false
				return
			}

			// Pass UI state directly as JSON to CLI
			const payloadObj = {
				workspace_id: workspace,
				repo_url_resource_path: gitRepoResourcePath,
				branch_to_push: _branchName,
				dry_run: true,
				settings_json: JSON.stringify(uiState)
			}

			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: payloadObj,
				skipPreprocessor: true
			})
			previewJobId = jobId
			previewJobStatus = 'running'

			// Wait for job completion (polling)
			let jobSuccess = false
			await tryEvery({
				tryCode: async () => {
					const testResult = await JobService.getCompletedJob({
						workspace,
						id: jobId
					})
					jobSuccess = !!testResult.success
				},
				timeoutCode: async () => {
					try {
						await JobService.cancelQueuedJob({
							workspace,
							id: jobId,
							requestBody: {
								reason: 'Preview job timed out after 15s'
							}
						})
					} catch (err) {
						console.error(err)
					}
				},
				interval: 500,
				timeout: 15000
			})

			if (jobSuccess) {
				const result = await JobService.getCompletedJobResult({
					workspace,
					id: jobId
				})
				console.log('Preview result:', result)

				// Convert new CLI format to expected format
				const added: string[] = []
				const deleted: string[] = []
				const modified: string[] = []

				if (result && result.changes && Array.isArray(result.changes)) {
					for (const change of result.changes) {
						if (change.type === 'added') {
							added.push(change.path)
						} else if (change.type === 'deleted') {
							deleted.push(change.path)
						} else if (change.type === 'modified') {
							modified.push(change.path)
						}
					}
				}

				previewResult = { added, deleted, modified }
				previewJobStatus = 'success'
			} else {
				previewResult = undefined
				previewJobStatus = 'failure'
			}
		} catch (error) {
			console.error('Failed to preview changes:', error)
			previewResult = undefined
			previewJobStatus = 'failure'
		} finally {
			isPreviewLoading = false
		}
	}

	async function initializeRepo() {
		const workspace = $workspaceStore
		if (!workspace) return
		console.log('Initializing repo:', gitRepoResourcePath, 'in workspace:', workspace)
		isInitializing = true
		initResult = null
		pushJobId = null
		pushJobStatus = undefined
		try {
			// Pass UI state directly as JSON to CLI
			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: {
					workspace_id: workspace,
					repo_url_resource_path: gitRepoResourcePath,
					branch_to_push: _branchName,
					settings_json: JSON.stringify(uiState)
				},
				skipPreprocessor: true
			})
			pushJobId = jobId
			pushJobStatus = 'running'

			let jobSuccess = false
			await tryEvery({
				tryCode: async () => {
					const testResult = await JobService.getCompletedJob({
						workspace,
						id: jobId
					})
					jobSuccess = !!testResult.success
				},
				timeoutCode: async () => {
					try {
						await JobService.cancelQueuedJob({
							workspace,
							id: jobId,
							requestBody: {
								reason: 'Push job timed out after 5s'
							}
						})
					} catch (err) {
						console.error(err)
					}
				},
				interval: 500,
				timeout: 10000
			})

			pushJobStatus = jobSuccess ? 'success' : 'failure'
			initResult = {
				success: jobSuccess,
				message: jobSuccess ? undefined : 'Failed to initialize repository.'
			}

			// Reset popover state after successful push
			if (jobSuccess) {
				setTimeout(() => {
					previewResult = undefined
					pushJobId = null
					pushJobStatus = undefined
					initResult = null
					initGitRepoPopover?.close()
				}, 1500) // Small delay to show success state
			}
		} catch (error) {
			console.error('Failed to initialize repo:', error)
			pushJobStatus = 'failure'
			initResult = { success: false, message: 'Failed to initialize repository.' }
		} finally {
			isInitializing = false
		}
	}
</script>

<Popover
	bind:this={initGitRepoPopover}
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
			onclick={initGitRepoPopover?.open}
			startIcon={{ icon: UploadCloud }}
		>
			Push workspace to Git repo
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content" let:close>
		<div class="flex flex-col gap-4">
			<div class="flex flex-col gap-2">
				<h3 class="text-lg font-semibold">Push workspace to Git repository</h3>
				<div class="prose max-w-none text-2xs text-tertiary">
					This action will push all workspace objects that match your current filter settings to the
					selected branch in your Git repository. <span
						class="text-orange-600 flex items-center gap-1"
						><AlertTriangle size={14} /> Any existing content in the branch will be replaced with the
						filtered workspace content.</span
					>

					<!-- Collapsible CLI Info Section -->
					<div class="mt-2 border rounded-md">
						<button
							class="w-full flex items-center justify-between p-1.5 bg-surface-secondary hover:bg-surface-hover"
							onclick={() => (isCliInfoExpanded = !isCliInfoExpanded)}
						>
							<span class="font-medium flex items-center gap-2">
								<Terminal size={14} />
								Windmill CLI to pull from Windmill and push to git
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
wmill workspace add {$workspaceStore} {$workspaceStore} {`${$page.url.protocol}//${$page.url.hostname}/`}
wmill init
# adjust wmill.yaml file configuraton as needed
wmill sync pull
git add -A
git commit -m 'Initial commit'
git push</code
										></pre
									>
								</div>
							</div>
						{/if}
					</div>
				</div>
			</div>

			<div class="flex flex-col gap-2">
				<label for="branch-name" class="text-sm font-medium">Push to new branch (optional)</label>
				<div class="prose max-w-none text-2xs text-tertiary">
					Enter a new branch name to push to (e.g so you can merge back into main with a pull
					request). If left blank, the default branch from the git repository resource will be used.
				</div>
				<div class="flex flex-col w-1/4">
					<input
						id="branch-name"
						type="text"
						bind:value={_branchName}
						class="border rounded px-2 py-1"
					/>
				</div>
			</div>

			{#if previewResult}
				<GitDiffPreview {previewResult} />
			{/if}

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

			{#if pushJobId}
				<div class="flex items-center gap-2 text-xs text-tertiary">
					{#if pushJobStatus === 'running'}
						<Loader2 class="animate-spin" size={14} />
					{:else if pushJobStatus === 'success'}
						<CheckCircle2 size={14} class="text-green-600" />
					{:else if pushJobStatus === 'failure'}
						<XCircle size={14} class="text-red-700" />
					{/if}
					Push job:
					<a
						target="_blank"
						class="underline"
						href={`/run/${pushJobId}?workspace=${$workspaceStore}`}
					>
						{pushJobId}
					</a>
				</div>
			{/if}

			<!-- Action row: Cancel on left, Preview/Confirm on right -->
			<div class="flex justify-between items-center mt-4">
				<Button
					color="light"
					size="xs"
					on:click={() => {
						previewResult = undefined
						previewJobId = null
						previewJobStatus = undefined
						pushJobId = null
						pushJobStatus = undefined
						initResult = null
						close()
					}}
					disabled={isPreviewLoading || isInitializing}
				>
					Cancel
				</Button>
				<div class="flex gap-2">
					{#if !previewResult}
						<Button
							size="xs"
							on:click={previewChanges}
							disabled={isPreviewLoading || isInitializing}
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
							disabled={isPreviewLoading || isInitializing}
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
								on:click={initializeRepo}
								disabled={isPreviewLoading || isInitializing}
								startIcon={{ icon: Save }}
								title="Initialize Git Repo"
							>
								Push
							</Button>
						{/if}
					{/if}
				</div>
			</div>

			{#if initResult?.message}
				<div class="mt-2">
					<Alert
						type={initResult.success ? 'success' : 'error'}
						title={initResult.success ? 'Success' : 'Error'}
						size="xs"
					>
						{initResult.message}
					</Alert>
				</div>
			{/if}
		</div>
	</svelte:fragment>
</Popover>
