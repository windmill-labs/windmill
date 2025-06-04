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
	import yaml from 'js-yaml'
	import { page } from '$app/stores'

	let { gitRepoResourcePath, uiState, onFilterUpdate } = $props<{
		gitRepoResourcePath: string
		uiState: {
			include_path: string[]
			include_type: string[]
		}
		onFilterUpdate: (filters: { yaml: string }) => void
	}>()

	type PreviewResult = {
		added: string[]
		deleted: string[]
		modified: string[]
		repoWmillYaml?: string
		yamlModified?: boolean
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
				const result = await JobService.getCompletedJobResult({ workspace, id: jobId }) as PreviewResult
				console.log('Preview result:', result)

				// For full sync mode, just use the CLI results directly
				// The CLI already handles wmill.yaml changes with --include-wmill-yaml flag
				previewResult = {
					...result,
					yamlModified: false // Don't add redundant YAML detection
				}
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
		if (!workspace || !previewResult?.repoWmillYaml) return

		console.log('Pulling from repo:', gitRepoResourcePath)
		isPulling = true
		jobStatus = { id: null, status: undefined, type: 'pull' }

		try {
			const yamlObj = yaml.load(previewResult.repoWmillYaml) as any
			if (!yamlObj || typeof yamlObj !== 'object') throw new Error('Invalid YAML')

			// Convert YAML config to UI state format for JSON approach
			const uiState = {
				include_path: yamlObj.includes || ['f/**'],
				include_type: [] as string[]
			}

			// Convert YAML flags back to include_type array
			const includeTypes = ['script', 'flow', 'app', 'folder'] // Always include core types
			if (!yamlObj.skipResourceTypes) includeTypes.push('resourcetype')
			if (!yamlObj.skipResources) includeTypes.push('resource')
			if (!yamlObj.skipVariables) includeTypes.push('variable')
			if (!yamlObj.skipSecrets && !yamlObj.skipVariables) includeTypes.push('secret')
			if (yamlObj.includeSchedules) includeTypes.push('schedule')
			if (yamlObj.includeTriggers) includeTypes.push('trigger')
			if (yamlObj.includeUsers) includeTypes.push('user')
			if (yamlObj.includeGroups) includeTypes.push('group')

			uiState.include_type = includeTypes

			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitSyncPush,
				requestBody: {
					payload: JSON.stringify({
						pusher: {
							name: 'workspace settings: windmill-pull'
						},
						ref: ''
					}),
					skip_secrets: yamlObj.skipSecrets ?? true,
					skip_variables: yamlObj.skipVariables ?? true,
					skip_resources: yamlObj.skipResources ?? true,
					skip_resource_types: yamlObj.skipResourceTypes ?? true,
					include_schedules: yamlObj.includeSchedules ?? false,
					include_users: yamlObj.includeUsers ?? false,
					include_groups: yamlObj.includeGroups ?? false,
					include_settings: yamlObj.includeSettings ?? false,
					include_key: yamlObj.includeKey ?? false,
					includes: yamlObj.includes?.join(',') ?? '',
					excludes: yamlObj.excludes?.join(',') ?? '',
					message: '"Pull from Git repository"',
					repo_resource_path: gitRepoResourcePath
				},
				skipPreprocessor: true
			})

			jobStatus = { id: jobId, status: 'running', type: 'pull' }
			const success = await handleJobCompletion(jobId, workspace)
			jobStatus.status = success ? 'success' : 'failure'

			if (success) {
				onFilterUpdate({ yaml: previewResult.repoWmillYaml })
				sendUserToast('Successfully pulled workspace content from repository')
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
					This action will pull all workspace objects from your Git repository according to the filters set in the Git repository wmill.yaml file and apply does filter settings to the workspace.
					<span class="text-orange-600 flex items-center gap-1">
						<AlertTriangle size={14} /> This will overwrite your current workspace content and Git sync filter settings with the content from the Git repository.
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
									Not familiar with Windmill CLI? <a href="https://www.windmill.dev/docs/advanced/cli/sync" class="text-blue-500 hover:text-blue-600 underline" target="_blank" rel="noopener noreferrer">Check out the docs</a>
								</div>
								<div class="font-mono text-2xs">
								<pre class="overflow-auto max-h-60"><code>npm install -g windmill-cli
# Clone your git repository
git clone $REPO_URL
cd $REPO_NAME
# Configure Windmill CLI
wmill workspace add {$workspaceStore} {$workspaceStore} {`${$page.url.protocol}//${$page.url.hostname}/`}
# Push the content to Windmill
wmill sync push --yes
# Optional: add --skip-secrets --skip-variables --skip-resources flags as needed</code></pre>
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
						previewResult = undefined;
						jobStatus = { id: null, status: undefined, type: 'preview' };
						close();
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
						{#if previewResult.added?.length || previewResult.deleted?.length || previewResult.modified?.length || previewResult.yamlModified}
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
