<script lang="ts">
	import { untrack } from 'svelte'
	import { CheckCircle2, Loader2, RotateCw, XCircle } from 'lucide-svelte'
	import { Alert, Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import EEOnly from '$lib/components/EEOnly.svelte'
	import GitSyncFilterSettings from '$lib/components/workspaceSettings/GitSyncFilterSettings.svelte'
	import { GitSyncService, ResourceService } from '$lib/gen'
	import { enterpriseLicense, userWorkspaces, workspaceStore } from '$lib/stores'
	import { apiErrorMessage } from '$lib/utils'
	import { getGitSyncContext } from '../GitSyncContext.svelte'
	import GitSyncModeDisplay from '../GitSyncModeDisplay.svelte'
	import { applyNewConnectionDefaults } from './connectionDefaults'

	type Props = {
		/** The unsaved repository this step configures, in the git sync context. */
		idx: number
		mode: 'sync' | 'promotion'
	}

	let { idx, mode }: Props = $props()

	const ctx = getGitSyncContext()
	const repo = $derived(ctx.getRepository(idx))

	const currentWorkspace = $derived($userWorkspaces?.find((w) => w.id === $workspaceStore))
	const isFork = $derived(
		($workspaceStore?.startsWith('wm-fork-') ?? false) || !!currentWorkspace?.parent_workspace_id
	)

	let managedCredential = $state(false)
	let targetBranch: string | undefined = $state(undefined)

	async function loadResourceFacts(path: string) {
		const workspace = $workspaceStore
		if (!workspace) return
		const [resource, origin] = await Promise.all([
			ResourceService.getResource({ workspace, path }).catch(() => undefined),
			// EE-only route: absent means Windmill holds no credential.
			GitSyncService.getCredentialOrigin({ workspace, path }).catch(() => undefined)
		])
		const isGithubApp = (resource?.value as any)?.is_github_app === true
		managedCredential = isGithubApp || origin?.origin !== undefined
		if (repo) {
			applyNewConnectionDefaults(repo, {
				mode,
				managedCredential,
				isGithubApp,
				isFork,
				ee: !!$enterpriseLicense
			})
			targetBranch = await ctx.getTargetBranch(repo).catch(() => undefined)
		}
	}

	async function detect() {
		try {
			await ctx.detectRepository(idx)
		} catch (e) {
			if (repo) {
				repo.detectionState = 'error'
				repo.detectionError = apiErrorMessage(e)
			}
		}
	}

	$effect(() => {
		const path = repo?.git_repo_resource_path
		if (!path) return
		untrack(() => {
			void loadResourceFacts(path)
			if (!repo?.detectionState || repo.detectionState === 'idle') void detect()
		})
	})

	// Applied to every fork of this workspace, so they are the parent's to set — a fork has
	// neither toggle, matching the repository card.
	function setSyncForks(v: boolean) {
		if (repo?.auto_pull) repo.auto_pull = { ...repo.auto_pull, sync_forks: v }
	}
	function setForkOpenPrs(v: boolean) {
		if (repo) repo.fork_open_prs = v
	}

	function setAutoPull(enabled: boolean) {
		if (!repo) return
		repo.auto_pull = enabled
			? {
					...(repo.auto_pull ?? {}),
					enabled: true,
					mode: repo.auto_pull?.mode ?? 'auto',
					sync_forks: repo.auto_pull?.sync_forks ?? true
				}
			: repo.auto_pull
				? { ...repo.auto_pull, enabled: false }
				: undefined
	}
</script>

{#if repo}
	<div class="flex flex-col gap-4 h-full">
		{#if !repo.detectionState || repo.detectionState === 'idle' || repo.detectionState === 'loading'}
			<div class="flex-1 flex flex-col items-center justify-center gap-2 text-sm text-hint">
				<Loader2 size={36} class="animate-spin" />
				Checking the repository...
			</div>
		{:else if repo.detectionState === 'error'}
			<Alert type="error" size="xs" title="Could not read the repository">
				<div class="flex flex-col items-start gap-2">
					<span>{repo.detectionError || 'The check failed.'}</span>
					<span> Check the URL and the token, or go back and connect the repository again. </span>
					<Button
						variant="default"
						unifiedSize="xs"
						startIcon={{ icon: RotateCw }}
						onClick={() => void detect()}
					>
						Retry
					</Button>
				</div>
			</Alert>
		{:else if repo.detectionState === 'no-wmill'}
			<Alert type="info" size="xs" bgClass="border-0" title="New repository">
				The repository has no Windmill configuration yet. Choose what to sync, then initialize it
				with the current content of this workspace.
			</Alert>
		{:else if repo.detectionState === 'has-wmill'}
			<Alert type="success" size="xs" bgClass="border-0" title="Existing Windmill repository">
				The repository already holds a Windmill configuration. Its sync settings are loaded below.
			</Alert>
		{/if}

		{#if repo.detectionState === 'no-wmill' || repo.detectionState === 'has-wmill'}
			<GitSyncFilterSettings
				git_repo_resource_path={repo.git_repo_resource_path}
				bind:include_path={repo.settings.include_path}
				bind:include_type={repo.settings.include_type}
				bind:exclude_types_override={repo.exclude_types_override}
				isLegacyRepo={false}
				bind:excludes={repo.settings.exclude_path}
				bind:extraIncludes={repo.settings.extra_include_path}
				isInitialSetup={repo.detectionState === 'no-wmill'}
				requiresMigration={false}
				useIndividualBranch={repo.use_individual_branch}
			>
				{#snippet subtitle()}
					<GitSyncModeDisplay {mode} {targetBranch} repository={repo} />
				{/snippet}
			</GitSyncFilterSettings>

			{#if mode === 'promotion'}
				<Toggle
					bind:checked={repo.group_by_folder}
					options={{
						right: 'Group all changes from the same folder in the same branch',
						rightTooltip:
							'Instead of creating a branch per item, Windmill creates a branch per folder containing the items being deployed.'
					}}
				/>
			{:else if !isFork}
				<div class="flex flex-col gap-1">
					<Toggle
						checked={repo.auto_pull?.enabled ?? false}
						disabled={!$enterpriseLicense}
						options={{
							right: 'Automatically deploy changes from Git',
							rightTooltip:
								'Windmill deploys new commits from the tracked branch into this workspace.'
						}}
						on:change={(e) => setAutoPull(e.detail)}
					>
						{#snippet right()}
							{#if !$enterpriseLicense}<EEOnly />{/if}
						{/snippet}
					</Toggle>
					{#if repo.auto_pull?.enabled}
						<span class="text-2xs text-secondary">
							{managedCredential
								? 'New commits are delivered instantly by webhook, with polling as a fallback.'
								: 'The branch is checked for new commits about every minute.'}
						</span>
					{/if}
					<Toggle
						disabled={!repo.auto_pull?.enabled}
						checked={!!(repo.auto_pull?.enabled && repo.auto_pull?.sync_forks)}
						options={{
							right: 'Automatically sync forks with git branches',
							rightTooltip: repo.auto_pull?.enabled
								? "When a fork's wm-fork/** branch changes in the repository (for example after merging the tracked branch into it), Windmill deploys those commits into the fork workspace. Configured once here, applied to every fork of this workspace."
								: 'Requires automatic deploy from Git to be enabled above.'
						}}
						on:change={(e) => setSyncForks(e.detail)}
					/>
					{#if managedCredential}
						<Toggle
							checked={repo.fork_open_prs ?? false}
							disabled={!$enterpriseLicense}
							options={{
								right: 'Open a pull request when an item is deployed in a fork',
								rightTooltip:
									"After an item deployed in a fork is pushed to the fork's branch (wm-fork/**, or the dev branch for a dev workspace), Windmill opens a pull request to the tracked branch of the shared repository. Runs from the deploy itself, so it works without inbound webhooks."
							}}
							on:change={(e) => setForkOpenPrs(e.detail)}
						>
							{#snippet right()}
								{#if !$enterpriseLicense}<EEOnly />{/if}
							{/snippet}
						</Toggle>
					{/if}
				</div>
			{/if}
		{/if}

		{#if repo.detectionState !== 'no-wmill' && repo.detectionState !== 'has-wmill'}
			<!-- Holds its row from the start: the job id only arrives once the check job has been
			     started, and letting the row appear then shifts everything above it. -->
			<div class="mt-auto flex items-center gap-2 min-h-4 text-2xs text-secondary">
				{#if repo.detectionJobId}
					{#if repo.detectionJobStatus === 'success'}
						<CheckCircle2 size={12} class="text-green-600" />
					{:else if repo.detectionJobStatus === 'failure'}
						<XCircle size={12} class="text-red-700" />
					{/if}
					<span class="text-hint">Check job:</span>
					<a
						target="_blank"
						class="underline"
						href={`/run/${repo.detectionJobId}?workspace=${$workspaceStore}`}
					>
						{repo.detectionJobId}
					</a>
				{/if}
			</div>
		{/if}
	</div>
{/if}
