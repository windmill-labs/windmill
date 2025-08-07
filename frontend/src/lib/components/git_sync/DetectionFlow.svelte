<script lang="ts">
	import { FileSearch, Save, Loader2, CheckCircle2, XCircle } from 'lucide-svelte'
	import { Button, Alert } from '$lib/components/common'
	import { getGitSyncContext } from './GitSyncContext.svelte'
	import GitSyncFilterSettings from '$lib/components/workspaceSettings/GitSyncFilterSettings.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'

	let { idx } = $props<{ idx: number }>()

	const gitSyncContext = getGitSyncContext()
	const repo = $derived(gitSyncContext.getRepository(idx))

	async function handleDetect() {
		try {
			await gitSyncContext.detectRepository(idx)
		} catch (error: any) {
			console.error('Detection failed:', error)
			sendUserToast('Detection failed: ' + error.message, true)
		}
	}

	async function handleInitialize() {
		if (!repo || repo.detectionState !== 'no-wmill') return

		try {
			// Show push modal for initialization
			gitSyncContext.showPushModal(idx)
		} catch (error: any) {
			console.error('Failed to initialize repository:', error)
			sendUserToast('Failed to initialize repository: ' + error.message, true)
		}
	}

	async function handleSaveConnection() {
		if (!repo || repo.detectionState !== 'has-wmill') return

		try {
			await gitSyncContext.saveRepository(idx)
			sendUserToast('Git sync connection saved successfully')
		} catch (error: any) {
			console.error('Failed to save connection:', error)
			sendUserToast('Failed to save connection: ' + error.message, true)
		}
	}
</script>

{#if repo}
	<div class="space-y-4">
		{#if !repo.detectionState || repo.detectionState === 'idle'}
			<!-- Step 1: Show toggles first, then check button -->
			<div class="space-y-3">
				<Toggle
					disabled={!repo.git_repo_resource_path}
					bind:checked={repo.use_individual_branch}
					options={{
						left: 'Sync mode',
						leftTooltip: 'Changes will be committed directly to the branch',
						right: 'Promotion mode',
						rightTooltip: "Changes will be made to a new branch per deployed object (prefixed with 'wm_deploy/')"
					}}
				/>

				{#if repo.use_individual_branch}
					<Toggle
						disabled={!repo.git_repo_resource_path}
						bind:checked={repo.group_by_folder}
						options={{
							right: 'Group all changes from same folder in the same branch',
							rightTooltip: 'Instead of creating a branch per object, Windmill will create a branch per folder containing objects being deployed.'
						}}
					/>
				{/if}
			</div>

			<!-- Check repo settings button -->
			<div class="flex justify-start">
				<Button
					color="primary"
					variant="border"
					size="sm"
					onclick={handleDetect}
					startIcon={{ icon: FileSearch }}
				>
					Check repo settings
				</Button>
			</div>
		{:else if repo.detectionState === 'loading'}
			<!-- Loading state -->
			<div class="flex items-center gap-2">
				<Loader2 size={16} class="animate-spin" />
				<span class="text-sm">Checking repository...</span>
			</div>
		{:else if repo.detectionState === 'no-wmill'}
			<!-- No wmill.yaml found - new repository -->
			<Alert type="info" title="Uninitialized Windmill repository found" class="mb-2">
				No git sync configuration found. Configure your sync settings below.
			</Alert>

			<GitSyncFilterSettings
				git_repo_resource_path={repo.git_repo_resource_path}
				bind:include_path={repo.settings.include_path}
				bind:include_type={repo.settings.include_type}
				bind:exclude_types_override={repo.exclude_types_override}
				isLegacyRepo={false}
				bind:excludes={repo.settings.exclude_path}
				bind:extraIncludes={repo.settings.extra_include_path}
				isInitialSetup={true}
				requiresMigration={false}
				useIndividualBranch={repo.use_individual_branch}
			/>

			<!-- Display mode settings as prominent text -->
			<div class="text-base">
				{#if repo.use_individual_branch}
					<div><span class="font-bold">Promotion:</span> Creating branches whose promotion target is main</div>
					{#if repo.group_by_folder}
						<div class="text-sm text-tertiary mt-1">Grouped by folder</div>
					{/if}
				{:else}
					<div>Sync: <span class="font-bold">Syncing back to branch main</span></div>
				{/if}
			</div>

			<!-- Initialize button -->
			<div class="flex justify-start">
				<Button
					size="md"
					onclick={handleInitialize}
					startIcon={{ icon: Save }}
				>
					Initialize Git repository
				</Button>
			</div>
		{:else if repo.detectionState === 'has-wmill'}
			<!-- wmill.yaml found - existing repository -->
			<Alert type="success" title="Existing Windmill repository found" class="mb-2">
				Found existing git sync configuration. Settings loaded from repository.
			</Alert>

			<GitSyncFilterSettings
				git_repo_resource_path={repo.git_repo_resource_path}
				bind:include_path={repo.settings.include_path}
				bind:include_type={repo.settings.include_type}
				bind:exclude_types_override={repo.exclude_types_override}
				isLegacyRepo={false}
				bind:excludes={repo.settings.exclude_path}
				bind:extraIncludes={repo.settings.extra_include_path}
				isInitialSetup={false}
				requiresMigration={false}
				useIndividualBranch={repo.use_individual_branch}
			/>

			<!-- Display mode settings as prominent text -->
			<div class="text-base">
				{#if repo.use_individual_branch}
					<div><span class="font-bold">Promotion:</span> Creating branches whose promotion target is main</div>
					{#if repo.group_by_folder}
						<div class="text-sm text-tertiary mt-1">Grouped by folder</div>
					{/if}
				{:else}
					<div>Sync: <span class="font-bold">Syncing back to branch main</span></div>
				{/if}
			</div>

			<!-- Save connection button -->
			<div class="flex justify-start">
				<Button
					size="md"
					onclick={handleSaveConnection}
					startIcon={{ icon: Save }}
				>
					Save connection
				</Button>
			</div>
		{:else if repo.detectionState === 'error'}
			<!-- Error state -->
			<Alert type="error" title="Detection error" class="my-2">
				{repo.detectionError || 'Failed to check repository'}
			</Alert>
		{/if}

		<!-- Job status display -->
		{#if repo.detectionJobId && (repo.detectionState === 'loading' || repo.detectionState === 'error')}
			<div class="flex items-center gap-2 text-xs text-tertiary">
				{#if repo.detectionJobStatus === 'running'}
					<Loader2 class="animate-spin" size={14} />
				{:else if repo.detectionJobStatus === 'success'}
					<CheckCircle2 size={14} class="text-green-600" />
				{:else if repo.detectionJobStatus === 'failure'}
					<XCircle size={14} class="text-red-700" />
				{/if}
				Detection job:
				<a
					target="_blank"
					class="underline"
					href={`/run/${repo.detectionJobId}?workspace=${$workspaceStore}`}
				>
					{repo.detectionJobId}
				</a>
			</div>
		{/if}
	</div>
{/if}
