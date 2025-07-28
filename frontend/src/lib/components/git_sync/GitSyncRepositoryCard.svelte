<script lang="ts">
	import { Save, Trash, XCircle, CheckCircle2, RotateCw, RotateCcw, Download, Upload } from 'lucide-svelte'
	import { Button, Alert } from '$lib/components/common'
	import { getGitSyncContext } from './GitSyncContext.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import GitSyncFilterSettings from '$lib/components/workspaceSettings/GitSyncFilterSettings.svelte'
	import DetectionFlow from './DetectionFlow.svelte'
		import { sendUserToast } from '$lib/toast'
	import { fade } from 'svelte/transition'
	import { workspaceStore } from '$lib/stores'
	import hubPaths from '$lib/hubPaths.json'

	let { idx } = $props<{ idx: number }>()

	const gitSyncContext = getGitSyncContext()
	const repo = $derived(gitSyncContext.getRepository(idx))
	const validation = $derived(gitSyncContext.getValidation(idx))
	const gitSyncTestJob = $derived(gitSyncContext.gitSyncTestJobs?.[idx])

	// Compute already-used repository paths to exclude from picker
	const usedRepositoryPaths = $derived(
		gitSyncContext.repositories
			.map((r, i) => i !== idx ? r.git_repo_resource_path : null)
			.filter((path): path is string => Boolean(path?.trim()))
	)

	async function handleSave() {
		if (!repo) return

		try {
			await gitSyncContext.saveRepository(idx)
			sendUserToast('Repository settings updated')
		} catch (error: any) {
			console.error('Failed to save repository:', error)
			sendUserToast('Failed to save repository: ' + error.message, true)
		}
	}

	function handleRevert() {
		if (!repo) return
		try {
			gitSyncContext.revertRepository?.(idx)
			sendUserToast('Reverted repository settings')
		} catch (error: any) {
			console.error('Failed to revert repository:', error)
			sendUserToast('Failed to revert repository: ' + error.message, true)
		}
	}

	function handleRemove() {
		gitSyncContext.removeRepository(idx)
	}

	function runGitSyncTestJob() {
		if (gitSyncContext.runTestJob) {
			gitSyncContext.runTestJob(idx)
		}
	}

	function emptyString(str: string | undefined | null): boolean {
		return !str || str.trim() === ''
	}


</script>

{#if repo}
	<div class="rounded-lg shadow-sm border p-0 w-full mb-4">
		<!-- Card Header -->
		<div class="flex items-center justify-between min-h-10 px-4 py-1 border-b">
			<div class="flex items-center gap-2">
				<span class="font-semibold">Repository #{idx + 1}</span>
				<span class="text-xs text-tertiary pt-1 pl-8">
					{repo.git_repo_resource_path}
				</span>
			</div>
			<div class="flex items-center gap-2">
				{#if validation.hasChanges && validation.isValid && !repo.isUnsavedConnection}
					<Button
						size="xs"
						onclick={handleSave}
						startIcon={{ icon: Save }}
					>
						Save changes
					</Button>
					{#if gitSyncContext.initialRepositories[idx] && !repo.legacyImported}
						<Button
							color="light"
							size="xs"
							onclick={handleRevert}
							startIcon={{ icon: RotateCcw }}
						>
							Revert
						</Button>
					{/if}
				{/if}
				<button
					class="text-secondary hover:text-primary focus:outline-none"
					onclick={() => (repo.collapsed = !repo.collapsed)}
					aria-label="Toggle collapse"
				>
					{#if repo.collapsed}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="h-5 w-5"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M19 9l-7 7-7-7"
							/>
						</svg>
					{:else}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="h-5 w-5"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M5 15l7-7 7 7"
							/>
						</svg>
					{/if}
				</button>
				<button
					transition:fade|local={{ duration: 100 }}
					class="rounded-full p-2 bg-surface-secondary duration-200 hover:bg-surface-hover"
					aria-label="Remove repository"
					onclick={handleRemove}
				>
					<Trash size={14} />
				</button>
			</div>
		</div>
		{#if !repo.collapsed}
			<div class="px-4 py-2">
				<div class="flex mt-5 mb-1 gap-1">
					{#key repo}
						<div class="pt-1 font-semibold">Resource: </div>
						<ResourcePicker
							bind:value={repo.git_repo_resource_path}
							resourceType={'git_repository'}
							disabled={!repo.isUnsavedConnection}
							excludedValues={usedRepositoryPaths}
						/>
						{#if !emptyString(repo.git_repo_resource_path)}
							<Button
								disabled={emptyString(repo.script_path)}
								color="dark"
								onclick={runGitSyncTestJob}
								size="xs">Test connection</Button
							>
						{/if}
					{/key}
				</div>

				{#if !emptyString(repo.git_repo_resource_path)}
					<div class="flex mb-5 text-normal text-2xs gap-1">
						{#if validation.isDuplicate}
							<span class="text-red-600">This resource is already used by another repository.</span>
						{/if}
						{#if gitSyncTestJob && gitSyncTestJob.status !== undefined}
							{#if gitSyncTestJob.status === 'running'}
								<RotateCw size={14} class="animate-spin" />
							{:else if gitSyncTestJob.status === 'success'}
								<CheckCircle2 size={14} class="text-green-600" />
							{:else}
								<XCircle size={14} class="text-red-700" />
							{/if}
							Git sync resource checked via Windmill job
							<a
								target="_blank"
								href={`/run/${gitSyncTestJob.jobId}?workspace=${$workspaceStore}`}
							>
								{gitSyncTestJob.jobId}
							</a>WARNING: Only read permissions are verified.
						{/if}
					</div>

					{#if repo.legacyImported}
						<Alert type="warning" title="Legacy git sync settings imported">
							This repository was initialized from workspace-level legacy Git-Sync settings. Review the filters and press <b>Save</b> to migrate.
						</Alert>
					{/if}
					<div class="flex flex-col mt-5 mb-1 gap-4">
						{#if repo}
							{#if repo.script_path != hubPaths.gitSync}
								<Alert type="warning" title="Script version mismatch">
									The git sync version for this repository is not latest. Current: <a
										target="_blank"
										href="https://hub.windmill.dev/scripts/windmill/6943/sync-script-to-git-repo-windmill/9014/versions"
										>{repo.script_path}</a
									>, latest:
									<a
										target="_blank"
										href="https://hub.windmill.dev/scripts/windmill/6943/sync-script-to-git-repo-windmill/9014/versions"
										>{hubPaths.gitSync}</a
									>
									<div class="flex mt-2">
										<Button
											size="xs"
											color="dark"
											onclick={() => {
												if (repo) {
													repo.script_path = hubPaths.gitSync
												}
											}}
											>Update git sync script (require save git settings to be applied)</Button
										>
									</div>
								</Alert>
							{/if}
							{#if repo.isUnsavedConnection && !emptyString(repo.git_repo_resource_path)}
								<!-- Use DetectionFlow component -->
								<div class="mt-4">
									<DetectionFlow {idx} />
								</div>
							{:else}
								<!-- Existing saved connection flow -->
								<GitSyncFilterSettings
									git_repo_resource_path={repo.git_repo_resource_path}
									bind:include_path={repo.settings.include_path}
									bind:include_type={repo.settings.include_type}
									bind:exclude_types_override={repo.exclude_types_override}
									isLegacyRepo={repo.legacyImported}
									bind:excludes={repo.settings.exclude_path}
									bind:extraIncludes={repo.settings.extra_include_path}
									isInitialSetup={false}
									requiresMigration={repo.legacyImported}
								/>
							{/if}

							{#if !repo.isUnsavedConnection}
								<Toggle
									disabled={emptyString(repo.git_repo_resource_path)}
									bind:checked={repo.use_individual_branch}
									options={{
										right: 'Create one branch per deployed object',
										rightTooltip:
											"If set, Windmill will create a unique branch per object being pushed based on its path, prefixed with 'wm_deploy/'."
									}}
								/>

								<Toggle
									disabled={emptyString(repo.git_repo_resource_path) ||
										!repo.use_individual_branch}
									bind:checked={repo.group_by_folder}
									options={{
										right: 'Group deployed objects by folder',
										rightTooltip:
											'Instead of creating a branch per object, Windmill will create a branch per folder containing objects being deployed.'
									}}
								/>

								<div class="w-1/3 flex gap-2">
									<Button
										size="xs"
										color="dark"
										variant="border"
										onclick={() => gitSyncContext.showPullModal(idx)}
										startIcon={{ icon: Download }}
									>
										Pull from repo
									</Button>
									<Button
										size="xs"
										color="dark"
										variant="border"
										onclick={() => gitSyncContext.showPushModal(idx)}
										startIcon={{ icon: Upload }}
									>
										Push to repo
									</Button>
								</div>
							{/if}

						{/if}
					</div>
				{:else}
					<div class="text-xs text-tertiary pt-1 pl-8">Please select a Git repository resource.</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}
