<script lang="ts">
	import { getGitSyncContext } from './GitSyncContext.svelte'
	import PushWorkspaceModal from '$lib/components/git_sync/PushWorkspaceModal.svelte'
	import PullWorkspaceModal from '$lib/components/git_sync/PullWorkspaceModal.svelte'
	import GitSyncSuccessModal from '$lib/components/git_sync/GitSyncSuccessModal.svelte'
	import { sendUserToast } from '$lib/toast'

	const gitSyncContext = getGitSyncContext()

	function handlePushSuccess() {
		const pushModal = gitSyncContext.activeModals.push
		if (!pushModal) return

		const { idx, repo } = pushModal

		// If this was a repository initialization, auto-save the connection
		if (repo.isUnsavedConnection && repo.detectionState === 'no-wmill') {
			gitSyncContext.saveRepository(idx).then(() => {
				sendUserToast('Repository initialized and connection saved successfully')
			}).catch((error) => {
				sendUserToast('Repository initialized but failed to save connection: ' + error.message, true)
			})
		} else {
			sendUserToast('Successfully pushed to git repository')
		}

		gitSyncContext.closePushModal()
	}


	function handlePullSuccess() {
		sendUserToast('Successfully pulled from git repository')
		gitSyncContext.closePullModal()
	}

	function handleFilterUpdate(idx: number, filters: any) {
		// Update the repository settings in the context
		const repo = gitSyncContext.getRepository(idx)
		if (repo) {
			repo.settings = filters
		}
	}

	function handleSettingsSaved() {
		// Update initial state to reflect that current state has been saved externally
		gitSyncContext.initialRepositories.splice(0, gitSyncContext.initialRepositories.length, ...gitSyncContext.repositories.map(repo => ({ ...repo })))
		sendUserToast('Settings applied successfully')
	}


	async function handleSaveWithoutInit(idx: number) {
		try {
			await gitSyncContext.saveRepository(idx, true)
			sendUserToast('Connection saved successfully without initializing repository')
			gitSyncContext.closePushModal()
		} catch (error: any) {
			sendUserToast('Failed to save connection: ' + error.message, true)
		}
	}
</script>

<!-- Push Modal -->
{#if gitSyncContext.activeModals.push}
	{@const { idx, repo } = gitSyncContext.activeModals.push}
	{@const isNewConnection = repo.isUnsavedConnection && repo.detectionState === 'no-wmill'}
	<PushWorkspaceModal
		bind:open={gitSyncContext.activeModals.push.open}
		gitRepoResourcePath={repo.git_repo_resource_path}
		uiState={{
			include_path: repo.settings.include_path,
			exclude_path: repo.settings.exclude_path,
			extra_include_path: repo.settings.extra_include_path,
			include_type: repo.settings.include_type
		}}
		isNewConnection={isNewConnection}
		onSuccess={handlePushSuccess}
		onSaveWithoutInit={isNewConnection ? () => handleSaveWithoutInit(idx) : undefined}
	/>
{/if}

<!-- Pull Modal -->
{#if gitSyncContext.activeModals.pull}
	{@const { idx, repo, settingsOnly } = gitSyncContext.activeModals.pull}
	<PullWorkspaceModal
		bind:open={gitSyncContext.activeModals.pull.open}
		gitRepoResourcePath={repo.git_repo_resource_path}
		repoIndex={idx}
		currentGitSyncSettings={gitSyncContext}
		uiState={{
			include_path: repo.settings.include_path,
			exclude_path: repo.settings.exclude_path,
			extra_include_path: repo.settings.extra_include_path,
			include_type: repo.settings.include_type
		}}
		onFilterUpdate={(filters) => handleFilterUpdate(idx, filters)}
		onSettingsSaved={handleSettingsSaved}
		onSuccess={handlePullSuccess}
		{settingsOnly}
	/>
{/if}

<!-- Success Modal -->
{#if gitSyncContext.activeModals.success}
	<GitSyncSuccessModal
		bind:open={gitSyncContext.activeModals.success.open}
		savedWithoutInit={gitSyncContext.activeModals.success.savedWithoutInit}
	/>
{/if}
