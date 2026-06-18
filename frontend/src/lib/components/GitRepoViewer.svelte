<script lang="ts">
	import { JobService, HelpersService, ResourceService, SettingService } from '$lib/gen'
	import { untrack } from 'svelte'
	import { Alert, Button } from './common'
	import S3FilePickerInner from './S3FilePickerInner.svelte'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { Loader, Loader2, ChevronDown, ChevronRight, ExternalLink } from 'lucide-svelte'
	import { hubPaths } from '$lib/hub'
	import { sleep } from '$lib/utils'

	const CLONE_MARKER_FILE = '.windmill_clone_complete'
	const POLL_INTERVAL_MS = 1500
	const MAX_POLL_DURATION_MS = 30 * 60 * 1000 // 30 minutes

	let s3FilePicker: S3FilePickerInner | undefined = $state()

	let isLoadingCommitHash = $state(false)
	let isLoadingRepoClone = $state(false)
	let isCheckingPathExists = $state(false)
	let pathExists = $state<boolean | null>(null)
	let error = $state<string | null>(null)

	// Live job state for the running clone
	let runningJobId = $state<string | null>(null)
	let runningLogs = $state<string>('')
	let logsExpanded = $state<boolean>(false)
	let cancelRequested = $state<boolean>(false)

	interface Props {
		gitRepoResourcePath: string
		gitSshIdentity?: string[]
		commitHashInput?: string
	}

	let { gitRepoResourcePath, gitSshIdentity, commitHashInput = $bindable() }: Props = $props()

	let commitHash = $derived(commitHashInput)

	async function populateS3WithGitRepo() {
		const workspace = $workspaceStore
		if (!workspace) return

		const payload = {
			workspace,
			resource_path: gitRepoResourcePath,
			git_ssh_identity: gitSshIdentity,
			commit: commitHash
		}

		isLoadingRepoClone = true
		error = null
		runningLogs = ''
		cancelRequested = false
		logsExpanded = true

		let jobId: string
		try {
			jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.cloneRepoToS3forGitRepoViewer,
				requestBody: payload,
				skipPreprocessor: true
			})
		} catch (err: any) {
			error = `Failed to start clone job: ${err?.message ?? err}`
			isLoadingRepoClone = false
			return
		}
		runningJobId = jobId

		const startedAt = Date.now()
		let logOffset = 0
		let isCompleted = false

		while (!isCompleted) {
			if (cancelRequested) {
				try {
					await JobService.cancelQueuedJob({
						workspace,
						id: jobId,
						requestBody: { reason: 'cancelled by user from git repo viewer' }
					})
				} catch (_err) {}
				error = 'Repository clone cancelled.'
				break
			}

			if (Date.now() - startedAt > MAX_POLL_DURATION_MS) {
				try {
					await JobService.cancelQueuedJob({
						workspace,
						id: jobId,
						requestBody: { reason: 'git repo clone exceeded 30 min' }
					})
				} catch (_err) {}
				error = `Repository clone exceeded ${Math.round(
					MAX_POLL_DURATION_MS / 60000
				)} min and was cancelled.`
				break
			}

			try {
				const update = await JobService.getJobUpdates({
					workspace,
					id: jobId,
					running: true,
					logOffset
				})
				if (update.new_logs) {
					runningLogs += update.new_logs
				}
				if (typeof update.log_offset === 'number') {
					logOffset = update.log_offset
				}
				if (update.completed) {
					isCompleted = true
					break
				}
			} catch (_err) {
				// Transient API failure — keep polling.
			}

			await sleep(POLL_INTERVAL_MS)
		}

		if (isCompleted) {
			try {
				const completed = await JobService.getCompletedJob({ workspace, id: jobId })
				if (completed.success) {
					// Verify the marker so partial uploads aren't shown as complete.
					const s3Path = `gitrepos/${workspace}/${gitRepoResourcePath}/${commitHash}/`
					try {
						const check = await HelpersService.checkS3FolderExists({
							workspace,
							fileKey: s3Path,
							markerFile: CLONE_MARKER_FILE
						})
						if (check.exists) {
							pathExists = true
							logsExpanded = false
						} else {
							pathExists = false
							error =
								'Clone job finished but the completion marker is missing. The repository may have been synced with an older clone script — re-run to refresh.'
						}
					} catch (err: any) {
						error = `Failed to verify clone completion: ${err?.message ?? err}`
					}
				} else {
					const errMsg =
						(completed.result as any)?.error?.message ??
						(completed.result as any)?.error ??
						'Clone job failed without an error message.'
					error = typeof errMsg === 'string' ? errMsg : JSON.stringify(errMsg)
				}
			} catch (err: any) {
				error = `Failed to read clone job result: ${err?.message ?? err}`
			}
		}

		isLoadingRepoClone = false
	}

	function cancelClone() {
		cancelRequested = true
	}

	async function fetchCommitHash() {
		if (commitHash) return
		try {
			error = null
			isLoadingCommitHash = true
			const result = await ResourceService.getGitCommitHash({
				workspace: $workspaceStore!,
				path: gitRepoResourcePath,
				gitSshIdentity: gitSshIdentity?.join(',')
			})
			commitHashInput = result.commit_hash
		} catch (err: any) {
			error = `Failed to load git repository ${gitRepoResourcePath}: ${err.status} -  ${err.message || 'Unknown error'}: ${err.body}`
		} finally {
			isLoadingCommitHash = false
		}
	}

	async function checkS3Path() {
		if (!commitHash) return
		try {
			error = null
			isCheckingPathExists = true
			const s3Path = `gitrepos/${$workspaceStore}/${gitRepoResourcePath}/${commitHash}/`
			const pathCheck = await HelpersService.checkS3FolderExists({
				workspace: $workspaceStore!,
				fileKey: s3Path,
				markerFile: CLONE_MARKER_FILE
			})
			pathExists = pathCheck.exists && pathCheck.is_folder
		} catch (err: any) {
			error = `Failed to load git repository ${gitRepoResourcePath}: ${err.status} -  ${err.message || 'Unknown error'}: ${err.body}`
		} finally {
			isCheckingPathExists = false
		}
	}

	$effect(() => {
		gitRepoResourcePath
		untrack(() => fetchCommitHash())
	})

	$effect(() => {
		;[commitHash, gitRepoResourcePath]
		untrack(() => checkS3Path())
	})

	$effect(() => {
		;[s3FilePicker, commitHash, gitRepoResourcePath, pathExists]
		untrack(() => s3FilePicker?.reloadContent())
	})
</script>

{#if error && !isLoadingRepoClone}
	<div class="p-4 space-y-3">
		<Alert type="error" title="Error Loading Repository">
			<p class="whitespace-pre-wrap break-words">{error}</p>
			{#if runningJobId}
				<a
					class="inline-flex items-center gap-1 mt-2 text-sm underline"
					href={`${base}/run/${runningJobId}?workspace=${$workspaceStore}`}
					target="_blank"
					rel="noreferrer noopener"
				>
					View job logs <ExternalLink size={12} />
				</a>
			{/if}
		</Alert>
		{#if runningLogs}
			<details class="text-xs">
				<summary class="cursor-pointer text-secondary">Last clone logs</summary>
				<pre
					class="mt-2 p-2 bg-surface-secondary rounded max-h-64 overflow-auto whitespace-pre-wrap"
					>{runningLogs}</pre
				>
			</details>
		{/if}
		<Button onclick={populateS3WithGitRepo} color="blue">Retry</Button>
	</div>
{:else if isLoadingCommitHash}
	<div class="flex items-center gap-2 p-4">
		<Loader2 class="h-4 w-4 animate-spin" />
		<span class="text-secondary">Fetching latest commit hash...</span>
	</div>
{:else if isLoadingRepoClone}
	<div class="p-4 space-y-3">
		<div class="flex items-center justify-between gap-2">
			<div class="flex items-center gap-2">
				<Loader class="h-4 w-4 animate-spin" />
				<span class="text-secondary">Cloning repository...</span>
			</div>
			<div class="flex items-center gap-2">
				{#if runningJobId}
					<a
						class="inline-flex items-center gap-1 text-xs underline text-secondary"
						href={`${base}/run/${runningJobId}?workspace=${$workspaceStore}`}
						target="_blank"
						rel="noreferrer noopener"
					>
						Job <ExternalLink size={12} />
					</a>
				{/if}
				<Button size="xs" color="light" variant="border" onclick={cancelClone}>Cancel</Button>
			</div>
		</div>
		<button
			type="button"
			class="flex items-center gap-1 text-xs text-secondary hover:text-primary"
			onclick={() => (logsExpanded = !logsExpanded)}
		>
			{#if logsExpanded}
				<ChevronDown size={12} />
			{:else}
				<ChevronRight size={12} />
			{/if}
			Logs
		</button>
		{#if logsExpanded}
			<pre
				class="mt-1 p-2 bg-surface-secondary rounded text-xs max-h-64 overflow-auto whitespace-pre-wrap"
				>{runningLogs || '(waiting for output...)'}</pre
			>
		{/if}
	</div>
{:else if commitHash && isCheckingPathExists}
	<div class="flex items-center gap-2 p-4">
		<Loader2 class="h-4 w-4 animate-spin" />
		<span class="text-secondary">Checking repository availability...</span>
	</div>
{:else if commitHash && pathExists === false}
	<div class="p-4 space-y-3">
		<Alert type="warning" title="Repository Not Available">
			<p>The git repository content is not yet available in storage.</p>
		</Alert>
		<Button onclick={populateS3WithGitRepo} color="blue">Load Repository Contents</Button>
	</div>
{:else if commitHash && pathExists === true}
	{#key `${gitRepoResourcePath}-${commitHash}`}
		<S3FilePickerInner
			bind:this={s3FilePicker}
			readOnlyMode
			hideS3SpecificDetails
			rootPath={`gitrepos/${$workspaceStore}/${gitRepoResourcePath}/${commitHash}/`}
			listStoredFilesRequest={HelpersService.listGitRepoFiles}
			loadFilePreviewRequest={HelpersService.loadGitRepoFilePreview}
			testConnectionRequest={(async (_d) => {
				const bucketConfig: any = await SettingService.getGlobal({
					key: 'object_store_cache_config'
				})
				return SettingService.testObjectStorageConfig({
					requestBody: bucketConfig
				})
			}) as any}
			loadFileMetadataRequest={HelpersService.loadGitRepoFileMetadata}
		>
			{#snippet replaceUnauthorizedWarning()}
				<div class="mb-2">
					<Alert type="error" title="Cannot view git repo">
						<p>
							The git repo resource you are trying to access either doesn't exist or you don't have
							access to it. Make sure the resource path is correct and that you have visibility over
							the resource.
						</p>
					</Alert>
				</div>
			{/snippet}
		</S3FilePickerInner>
	{/key}
{/if}
