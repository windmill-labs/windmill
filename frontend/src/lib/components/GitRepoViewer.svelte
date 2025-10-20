<script lang="ts">
	import { JobService, HelpersService, ResourceService, SettingService } from '$lib/gen'
	import { untrack } from 'svelte'
	import { Alert, Button } from './common'
	import S3FilePickerInner from './S3FilePickerInner.svelte'
	import { workspaceStore } from '$lib/stores'
	import { Loader, Loader2 } from 'lucide-svelte'
	import { hubPaths } from '$lib/hub'
	import { tryEvery } from '$lib/utils'

	let s3FilePicker: S3FilePickerInner | undefined = $state()

	let isLoadingCommitHash = $state(false)
	let isLoadingRepoClone = $state(false)
	let isCheckingPathExists = $state(false)
	let pathExists = $state<boolean | null>(null)
	let error = $state<string | null>(null)

	interface Props {
		gitRepoResourcePath: string
		gitSshIdentity?: string[]
		commitHashInput?: string
	}

	let { gitRepoResourcePath, gitSshIdentity, commitHashInput = $bindable() }: Props = $props()

	let commitHash = $derived(commitHashInput);

	async function populateS3WithGitRepo() {
		const workspace = $workspaceStore
		if (!workspace) return

		const payload = {
			workspace: workspace,
			resource_path: gitRepoResourcePath,
			git_ssh_identity: gitSshIdentity,
			commit: commitHash,
		}

		isLoadingRepoClone = true
		const jobId = await JobService.runScriptByPath({
			workspace,
			path: hubPaths.cloneRepoToS3forGitRepoViewer,
			requestBody: payload,
			skipPreprocessor: true
		})

		let jobSuccess = false
		await tryEvery({
			tryCode: async () => {
				const testResult = await JobService.getCompletedJob({ workspace, id: jobId })
				jobSuccess = !!testResult.success
				if (jobSuccess) {
					await JobService.getCompletedJobResult({ workspace, id: jobId })
				} else {
					error = (testResult.result as any).error.message ?? "Failed to clone"
				}
			},
			timeoutCode: async () => {
				try {
					await JobService.cancelQueuedJob({
						workspace,
						id: jobId,
						requestBody: { reason: `job timed out after 60s` }
					})
				} catch (err) {}
			},
			interval: 1000,
			timeout: 60000
		})
		isLoadingRepoClone = false
		if (jobSuccess) {
			pathExists = true
		}
	}

	async function fetchGitRepoData() {
		try {
			error = null
			// Step 1: Fetch commit hash if not provided
			if (!commitHash) {
				isLoadingCommitHash = true
				error = null
				const result = await ResourceService.getGitCommitHash({
					workspace: $workspaceStore!,
					path: gitRepoResourcePath,
					gitSshIdentity: gitSshIdentity?.join(",")
				})

				commitHashInput = result.commit_hash
				isLoadingCommitHash = false
			}

			// Step 2: Check if S3 path exists
			if (commitHash) {
				isCheckingPathExists = true

				const s3Path = `gitrepos/${$workspaceStore}/${gitRepoResourcePath}/${commitHash}/`

				const pathCheck = await HelpersService.checkS3FolderExists({
					workspace: $workspaceStore!,
					fileKey: s3Path
				})

				pathExists = pathCheck.exists && pathCheck.is_folder
				isCheckingPathExists = false
			}
		} catch (err: any) {
			error = `Failed to load git repository ${gitRepoResourcePath}: ${err.status} -  ${err.message || 'Unknown error'}: ${err.body}`
			isLoadingCommitHash = false
			isCheckingPathExists = false
		}
	}

	$effect(() => {
		;[commitHashInput, gitRepoResourcePath]
		untrack(() => {
			fetchGitRepoData()
		})

	})

	$effect(() => {
		;[commitHash, pathExists, isCheckingPathExists, isLoadingCommitHash]
		untrack(() => {
			s3FilePicker?.open(undefined)
		})
	})
</script>

{#if error}
	<Alert type="error" title="Error Loading Repository">
		<p>{error}</p>
	</Alert>
{:else if isLoadingCommitHash}
	<div class="flex items-center gap-2 p-4">
		<Loader2 class="h-4 w-4 animate-spin" />
		<span class="text-secondary">Fetching latest commit hash...</span>
	</div>
{:else if isLoadingRepoClone}
	<div class="flex items-center gap-2 p-4">
		<Loader class="h-4 w-4 animate-spin" />
		<span class="text-secondary">Cloning repository...</span>
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
	<S3FilePickerInner
		bind:this={s3FilePicker}
		readOnlyMode
		hideS3SpecificDetails
		rootPath={`gitrepos/${$workspaceStore}/${gitRepoResourcePath}/${commitHash}/`}
		listStoredFilesRequest={HelpersService.listGitRepoFiles}
		loadFilePreviewRequest={HelpersService.loadGitRepoFilePreview}
		testConnectionRequest={(async (_d) => {
			const bucketConfig: any = await SettingService.getGlobal({ key: 'object_store_cache_config' })
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
{/if}
