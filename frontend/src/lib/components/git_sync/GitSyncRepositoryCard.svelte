<script lang="ts">
	import {
		Save,
		Trash,
		XCircle,
		CheckCircle2,
		RotateCw,
		RotateCcw,
		Download,
		Upload,
		Plus
	} from 'lucide-svelte'
	import { Button, Alert } from '$lib/components/common'
	import { getGitSyncContext } from './GitSyncContext.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import GitSyncFilterSettings from '$lib/components/workspaceSettings/GitSyncFilterSettings.svelte'
	import DetectionFlow from './DetectionFlow.svelte'
	import { sendUserToast } from '$lib/toast'
	import { fade } from 'svelte/transition'
	import { workspaceStore, userWorkspaces, enterpriseLicense } from '$lib/stores'
	import type { GitSyncRepository } from './GitSyncContext.svelte'
	import GitSyncModeDisplay from './GitSyncModeDisplay.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import EEOnly from '$lib/components/EEOnly.svelte'
	import { ResourceService, VariableService } from '$lib/gen'

	let {
		idx = null,
		isSecondary = false,
		isLegacy = false,
		variant = 'standard',
		mode = null,
		repository = null,
		onAdd = null,
		isCollapsible = true,
		showEmptyState = false,
		devPromotion = false
	} = $props<{
		idx?: number | null
		isSecondary?: boolean
		isLegacy?: boolean
		variant?: 'primary-sync' | 'primary-promotion' | 'secondary' | 'legacy' | 'standard'
		mode?: 'sync' | 'promotion' | null
		repository?: GitSyncRepository | null
		onAdd?: (() => void) | null
		isCollapsible?: boolean
		showEmptyState?: boolean
		// Dev workspace: this is the single inherited repo, and promotion is a
		// toggle on it (reuse prod's repo) rather than a separately-configured one.
		devPromotion?: boolean
	}>()

	const gitSyncContext = getGitSyncContext()
	const repo = $derived(repository || (idx !== null ? gitSyncContext.getRepository(idx) : null))
	const validation = $derived(idx !== null ? gitSyncContext.getValidation(idx) : null)
	const gitSyncTestJob = $derived(idx !== null ? gitSyncContext.gitSyncTestJobs?.[idx] : null)
	let confirmingDelete = $state(false)

	// Enable/disable automatic repo → workspace pulls, managing the optional
	// auto_pull object without binding into a possibly-undefined value.
	function setAutoPullEnabled(enabled: boolean) {
		if (!repo) return
		if (enabled) {
			repo.auto_pull = {
				...(repo.auto_pull ?? {}),
				enabled: true,
				mode: repo.auto_pull?.mode ?? 'auto',
				sync_forks: repo.auto_pull?.sync_forks ?? true
			}
		} else if (repo.auto_pull) {
			repo.auto_pull = { ...repo.auto_pull, enabled: false }
		}
	}

	// Parent-level fork auto-sync (phase 5). Configured on the parent workspace's
	// repo and applied to all of its forks, so hide it when the current workspace
	// is itself a fork.
	const currentWorkspaceData = $derived($userWorkspaces?.find((w) => w.id === $workspaceStore))
	// A fork or dev workspace: has a parent, or carries the wm-fork- id prefix
	// (which survives if the parent is deleted). Mirrors the backend/CLI rule.
	const isFork = $derived(
		($workspaceStore?.startsWith('wm-fork-') ?? false) ||
			!!currentWorkspaceData?.parent_workspace_id
	)
	// A dev workspace is a fork that DOES run promotion mode (per-item
	// wm_deploy/** branches into its parent), unlike a throwaway fork.
	const isDevWorkspace = $derived(!!currentWorkspaceData?.is_dev_workspace)
	function setSyncForks(v: boolean) {
		if (repo?.auto_pull) repo.auto_pull = { ...repo.auto_pull, sync_forks: v }
	}
	function setForkOpenPrs(v: boolean) {
		if (repo) repo.fork_open_prs = v
	}
	function setPromotionOpenPrs(v: boolean) {
		if (repo) repo.promotion_open_prs = v
	}
	// Dev-workspace promotion toggle: flips the inherited repo between sync mode
	// (deploys to the dev branch) and promotion mode (per-item wm_deploy/** PRs to
	// prod). Persists immediately — it's a mode switch on an already-saved repo.
	async function setDevPromotion(v: boolean) {
		if (!repo || idx === null) return
		const prevIndiv = repo.use_individual_branch
		const prevGbf = repo.group_by_folder
		repo.use_individual_branch = v
		if (!v) repo.group_by_folder = false
		try {
			await gitSyncContext.saveRepository(idx)
		} catch (e) {
			// The backend rejects promotion mode without an active EE plan; revert
			// the optimistic toggle instead of leaving it stuck on until reload.
			if (repo) {
				repo.use_individual_branch = prevIndiv
				repo.group_by_folder = prevGbf
			}
			sendUserToast(`Could not ${v ? 'enable' : 'disable'} Git promotion: ${e}`, true)
		}
	}
	async function setGroupByFolder(v: boolean) {
		if (!repo || idx === null) return
		const prev = repo.group_by_folder
		repo.group_by_folder = v
		try {
			await gitSyncContext.saveRepository(idx)
		} catch (e) {
			if (repo) repo.group_by_folder = prev
			sendUserToast(`Could not change promotion granularity: ${e}`, true)
		}
	}

	let targetBranch = $state<string | undefined>(undefined) // Default to main, will be updated when resource is available
	// The branch this fork workspace syncs with, mirroring the CLI/hub-script
	// naming: a dev workspace uses its environment-label branch verbatim
	// (dev/staging); a wm-fork-<slug> throwaway fork keeps only the slug.
	const forkBranch = $derived(
		currentWorkspaceData?.is_dev_workspace
			? (currentWorkspaceData?.dev_workspace_label ?? 'dev')
			: `wm-fork/${targetBranch ?? 'main'}/${($workspaceStore ?? '').replace(/^wm-fork-/, '')}`
	)
	let resourceInfo = $state<{ url?: string; error?: string } | null>(null)
	let loadingResourceInfo = $state(false)
	// Only GitHub App-backed repos can register webhooks; PAT repos poll only.
	let isGithubApp = $state(false)

	// Update target branch when repository changes
	$effect(() => {
		const abortController = new AbortController()

		if (repo?.git_repo_resource_path) {
			gitSyncContext
				.getTargetBranch(repo)
				.then((branch) => {
					if (!abortController.signal.aborted) {
						targetBranch = branch
					}
				})
				.catch((error) => {
					if (!abortController.signal.aborted) {
						console.warn('Failed to get target branch:', error)
					}
				})
		}

		return () => {
			abortController.abort()
		}
	})

	// Load resource info when resource path is set and connection is saved (disabled)
	$effect(() => {
		const abortController = new AbortController()

		async function loadResourceInfo() {
			if (repo?.git_repo_resource_path && $workspaceStore) {
				loadingResourceInfo = true
				resourceInfo = null
				// Clear stale app state up front so a resource change or a failed
				// fetch can't leave webhook/fork controls showing for the wrong repo.
				isGithubApp = false
				try {
					const resource = await ResourceService.getResource({
						workspace: $workspaceStore,
						path: repo.git_repo_resource_path
					})

					if (!abortController.signal.aborted && resource?.value) {
						// Extract git URL from resource value
						const value = resource.value as Record<string, any>
						isGithubApp = value?.is_github_app === true
						// A newly added sync connection defaults to pulling from Git only
						// when the repository is app-backed (instant webhook delivery).
						// Polling is opt-in for token repositories, and fork/dev workspaces
						// never get the parent-only defaults (the backend rejects them).
						// EE-only.
						if (
							repoMode === 'sync' &&
							repo.isUnsavedConnection &&
							isGithubApp &&
							!isFork &&
							$enterpriseLicense &&
							repo.auto_pull === undefined
						) {
							repo.auto_pull = { enabled: true, mode: 'auto', sync_forks: true }
						}
						// Promotion deploys push wm_deploy/** branches that exist to be
						// merged; without a PR the deploy is an orphaned branch. Default
						// the managed PR on where Windmill can open it (app-backed).
						// Fork PRs stay opt-in everywhere.
						if (
							repoMode === 'promotion' &&
							repo.isUnsavedConnection &&
							isGithubApp &&
							$enterpriseLicense &&
							repo.promotion_open_prs === undefined
						) {
							repo.promotion_open_prs = true
						}
						// Webhook with polling fallback is the only delivery for app repos.
						if (isGithubApp && repo.auto_pull?.mode === 'polling') {
							repo.auto_pull = { ...repo.auto_pull, mode: 'auto' }
						}
						let gitUrl = value?.url || value?.git_url

						if (gitUrl && typeof gitUrl === 'string') {
							// Check if the URL is a variable reference ($var:path)
							const varMatch = gitUrl.match(/^\$var:(.+)$/)

							if (varMatch) {
								const varPath = varMatch[1]
								try {
									// Attempt to fetch the variable value
									const variable = await VariableService.getVariable({
										workspace: $workspaceStore,
										path: varPath,
										decryptSecret: true
									})

									if (!abortController.signal.aborted && variable?.value) {
										gitUrl = variable.value
									} else {
										// Variable doesn't have a value or couldn't be fetched
										// Don't display anything
										resourceInfo = null
										return
									}
								} catch (error) {
									// Failed to fetch variable (permissions, not found, etc.)
									// Don't display anything
									if (!abortController.signal.aborted) {
										console.debug('Failed to fetch variable for git URL:', error)
										resourceInfo = null
									}
									return
								}
							}

							// Mask password in URL (supports https://user:password@host and https://token@host patterns)
							const maskedUrl = gitUrl.replace(
								/(https?:\/\/)([^:@]+)(:([^@]+))?@/,
								(match, protocol, user, colonPassword, password) => {
									if (password) {
										return `${protocol}${user}:${'*'.repeat(8)}@`
									}
									return `${protocol}${'*'.repeat(8)}@`
								}
							)
							resourceInfo = { url: maskedUrl }
						} else {
							resourceInfo = { error: 'Git URL not found in resource' }
						}
					}
				} catch (error) {
					if (!abortController.signal.aborted) {
						console.error('Failed to load resource info:', error)
						resourceInfo = { error: 'Failed to load resource info' }
					}
				} finally {
					if (!abortController.signal.aborted) {
						loadingResourceInfo = false
					}
				}
			} else {
				resourceInfo = null
				isGithubApp = false
			}
		}

		loadResourceInfo()

		return () => {
			abortController.abort()
		}
	})

	// Compute already-used repository paths to exclude from picker
	const usedRepositoryPaths = $derived(
		gitSyncContext.repositories
			.map((r, i) => (i !== idx ? r.git_repo_resource_path : null))
			.filter((path): path is string => Boolean(path?.trim()))
	)

	// Determine display title based on variant and legacy status
	const displayTitle = $derived(
		variant === 'primary-sync'
			? mode === 'sync'
				? 'Git Sync'
				: 'Git Promotion'
			: variant === 'primary-promotion'
				? 'Git Promotion'
				: isLegacy
					? 'Legacy promotion repository'
					: isSecondary
						? repo?.use_individual_branch
							? 'Secondary promotion repository'
							: 'Secondary sync repository'
						: `Repository #${(idx ?? 0) + 1}`
	)

	// Determine the actual mode based on repository configuration
	const repoMode = $derived<'sync' | 'promotion'>(
		variant === 'primary-promotion' || variant === 'legacy' || repo?.use_individual_branch
			? 'promotion'
			: 'sync'
	)

	// Determine display description based on variant and mode
	const targetOrDefaultBranch = $derived(targetBranch ? `'${targetBranch}'` : "repo's default")
	const displayDescription = $derived(
		variant === 'primary-sync' || variant === 'primary-promotion'
			? mode === 'sync'
				? `Deploys are committed to the ${targetOrDefaultBranch} branch, and new commits to it can deploy back into this workspace automatically`
				: mode === 'promotion'
					? `Each deploy in this workspace pushes its changes to a dedicated wm_deploy/** branch of the repository instead of committing to ${targetOrDefaultBranch} directly. Merging that branch into ${targetOrDefaultBranch} promotes the change: the workspace that syncs ${targetOrDefaultBranch} deploys it on merge, so set up Git Sync there. Windmill can open the pull request for each deploy branch (toggle below), or use the open-pr-on-commit workflow.`
					: null
			: null
	)

	const shouldShowEmptyState = $derived(
		showEmptyState || (!repo && (variant === 'primary-sync' || variant === 'primary-promotion'))
	)

	async function handleSave() {
		if (!repo || idx === null) return

		try {
			await gitSyncContext.saveRepository(idx)
			sendUserToast('Repository settings updated')
		} catch (error: any) {
			console.error('Failed to save repository:', error)
			sendUserToast('Failed to save repository: ' + error.message, true)
		}
	}

	function handleRevert() {
		if (!repo || idx === null) return
		try {
			gitSyncContext.revertRepository?.(idx)
			sendUserToast('Reverted repository settings')
		} catch (error: any) {
			console.error('Failed to revert repository:', error)
			sendUserToast('Failed to revert repository: ' + error.message, true)
		}
	}

	function initiateDelete() {
		confirmingDelete = true
	}

	async function confirmDelete() {
		if (idx === null) return
		try {
			await gitSyncContext.removeRepository(idx)
			sendUserToast('Repository connection removed successfully')
		} catch (error: any) {
			console.error('Failed to remove repository:', error)
			sendUserToast('Failed to remove repository: ' + error.message, true)
		} finally {
			confirmingDelete = false
		}
	}

	function cancelDelete() {
		confirmingDelete = false
	}

	function runGitSyncTestJob() {
		if (idx !== null && gitSyncContext.runTestJob) {
			gitSyncContext.runTestJob(idx)
		}
	}

	function emptyString(str: string | undefined | null): boolean {
		return !str || str.trim() === ''
	}

	function handlePullSettings() {
		if (idx !== null) {
			gitSyncContext.showPullModal(idx, true)
		}
	}
</script>

<!-- Shared snippets for reusable content -->

{#snippet headerActions()}
	{#if !isLegacy}
		{#if validation?.hasChanges && validation?.isValid && !repo.isUnsavedConnection}
			<Button size="xs" variant="accent" onclick={handleSave} startIcon={{ icon: Save }}>
				{repo.legacyImported ? 'Migrate and save' : 'Save changes'}
			</Button>
			{#if idx !== null && gitSyncContext.initialRepositories[idx] && !repo.legacyImported}
				<Button color="light" size="xs" onclick={handleRevert} startIcon={{ icon: RotateCcw }}>
					Revert
				</Button>
			{/if}
		{/if}
	{/if}
	{#if isCollapsible}
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
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
				</svg>
			{/if}
		</button>
	{/if}
	{#if !confirmingDelete}
		<div transition:fade|local={{ duration: 100 }}>
			<Button
				size="xs"
				variant="default"
				onclick={initiateDelete}
				startIcon={{ icon: Trash }}
				destructive
			>
				Delete
			</Button>
		</div>
	{:else}
		<div class="flex gap-1">
			<button
				transition:fade|local={{ duration: 100 }}
				class="px-3 py-1 text-xs bg-red-500 text-white rounded duration-200 hover:bg-red-600"
				onclick={confirmDelete}
			>
				Confirm delete
			</button>
			<button
				transition:fade|local={{ duration: 100 }}
				class="px-2 py-1 text-xs bg-surface-secondary rounded duration-200 hover:bg-surface-hover"
				onclick={cancelDelete}
			>
				<XCircle size={12} />
			</button>
		</div>
	{/if}
{/snippet}

{#snippet repositoryContent()}
	<div class="space-y-4">
		<!-- Resource Picker -->
		<div class="flex gap-2 items-center">
			<div class="font-semibold text-xs text-emphasis">Resource:</div>
			<div class="flex-1">
				<ResourcePicker
					bind:value={repo.git_repo_resource_path}
					resourceType={'git_repository'}
					disabled={!repo.isUnsavedConnection}
					excludedValues={usedRepositoryPaths}
				/>
			</div>
			{#if !emptyString(repo.git_repo_resource_path)}
				<Button
					disabled={emptyString(repo.git_repo_resource_path)}
					variant="default"
					onclick={runGitSyncTestJob}
					size="xs"
				>
					Test connection
				</Button>
			{/if}
		</div>

		<!-- Display resource info when disabled (saved connection) -->
		{#if !repo.isUnsavedConnection && repo.git_repo_resource_path}
			<div class="text-xs">
				{#if loadingResourceInfo}
					<div class="flex items-center gap-1 text-secondary">
						<RotateCw size={12} class="animate-spin" />
						<span>Loading resource info...</span>
					</div>
				{:else if resourceInfo?.url}
					<div class="flex items-center gap-2 text-secondary">
						<span class="text-xs text-secondary">Git URL:</span>
						<code class="bg-surface-secondary px-2 py-1 rounded text-primary"
							>{resourceInfo.url}</code
						>
					</div>
				{:else if resourceInfo?.error}
					<div class="text-red-600">{resourceInfo.error}</div>
				{/if}
			</div>
		{/if}

		{#if !emptyString(repo.git_repo_resource_path)}
			<!-- Validation and Test Status -->
			{#if validation?.isDuplicate}
				<div class="text-red-600 text-sm">
					This resource is already used by another repository.
				</div>
			{/if}
			{#if gitSyncTestJob && gitSyncTestJob.status !== undefined}
				<div class="flex text-sm gap-1 items-center">
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
						class="text-blue-500 hover:underline"
					>
						{gitSyncTestJob.jobId}
					</a>
					<span class="text-secondary">WARNING: Only read permissions are verified.</span>
				</div>
				{#if gitSyncTestJob.status === 'failure' && gitSyncTestJob.error}
					<div class="text-red-600 text-xs mt-1">
						Error: {gitSyncTestJob.error}
					</div>
				{/if}
			{/if}

			<!-- Warnings -->
			{#if repo.legacyImported}
				<Alert type="warning" title="Legacy git sync settings imported">
					This repository was initialized from workspace-level legacy Git-Sync settings. Review the
					filters and press <b>Save</b> to migrate.
				</Alert>
			{/if}

			{#if repo.script_path}
				<Alert type="warning" title="Pinned git sync script version">
					This repository uses a pinned sync script: <code>{repo.script_path}</code>. Switch to
					auto-managed to always use the latest version bundled with Windmill.
					<div class="flex mt-2">
						<Button
							size="xs"
							variant="accent"
							onclick={() => {
								if (repo) {
									repo.script_path = undefined
								}
							}}
						>
							Switch to auto-managed (requires save)
						</Button>
					</div>
				</Alert>
			{/if}

			<!-- Configuration -->
			{#if repo.isUnsavedConnection && !emptyString(repo.git_repo_resource_path) && idx !== null}
				<DetectionFlow {idx} mode={repoMode} />
			{:else}
				<GitSyncFilterSettings
					bind:git_repo_resource_path={repo.git_repo_resource_path}
					bind:include_path={repo.settings.include_path}
					bind:include_type={repo.settings.include_type}
					bind:exclude_types_override={repo.exclude_types_override}
					isLegacyRepo={repo.legacyImported}
					bind:excludes={repo.settings.exclude_path}
					bind:extraIncludes={repo.settings.extra_include_path}
					isInitialSetup={false}
					requiresMigration={repo.legacyImported}
					useIndividualBranch={repo.use_individual_branch}
				>
					{#snippet actions()}
						<Button size="md" onclick={handlePullSettings} startIcon={{ icon: Download }}>
							Pull settings
						</Button>
					{/snippet}
				</GitSyncFilterSettings>

				{#if !repo.isUnsavedConnection}
					{#if !emptyString(repo.git_repo_resource_path) && !repo.legacyImported}
						<!-- Direction 1: Windmill -> Git (commit/push on deploy) -->
						<div class="mt-4 border-t border-gray-200 pt-3 dark:border-gray-700">
							<div class="flex justify-between items-start gap-4">
								<div class="flex-1">
									<div class="text-sm font-semibold text-emphasis mb-1">
										Push to Git on deploy (Windmill → Git)
									</div>
									{#if !isFork || (isDevWorkspace && repoMode === 'promotion')}
										<GitSyncModeDisplay mode={repoMode} {targetBranch} repository={repo} active />
									{/if}
								</div>
								<Button
									size="xs"
									variant="default"
									onclick={() => idx !== null && gitSyncContext.showPushModal(idx)}
									startIcon={{ icon: Upload }}
								>
									Push to repo
								</Button>
							</div>
							{#if devPromotion && !repo.isUnsavedConnection}
								<div class="mt-2">
									<Toggle
										checked={repo.use_individual_branch ?? false}
										options={{
											right: 'Promote to prod via Git',
											rightTooltip:
												"Each deploy pushes a per-item wm_deploy/** branch to prod's repository, ready to open a pull request into its tracked branch, instead of committing to the dev branch. Enable automatic pull requests below to have Windmill open them. Reuses prod's repository, no separate setup."
										}}
										on:change={(e) => setDevPromotion(e.detail)}
									/>
									{#if repo.use_individual_branch}
										<div class="mt-2">
											<Toggle
												checked={repo.group_by_folder ?? false}
												options={{
													right: 'One pull request per folder',
													rightTooltip:
														"Group a folder's items into a single wm_deploy/** branch, ready as one pull request, instead of one branch per item."
												}}
												on:change={(e) => setGroupByFolder(e.detail)}
											/>
										</div>
									{/if}
								</div>
							{/if}
							{#if repoMode === 'promotion' && isGithubApp}
								<div class="mt-2">
									<Toggle
										checked={repo.promotion_open_prs ?? false}
										options={{
											right: 'Open a pull request for each deploy branch',
											rightTooltip:
												'After a deploy pushes its wm_deploy/** branch, Windmill opens a pull request to the target branch. Runs from the deploy itself, so it works without inbound webhooks.'
										}}
										on:change={(e) => setPromotionOpenPrs(e.detail)}
									/>
								</div>
							{:else if repoMode === 'promotion' && !repo.isUnsavedConnection}
								<div class="text-2xs text-secondary mt-2">
									To open a pull request for each deploy branch, set up the
									<a
										href="https://www.windmill.dev/docs/advanced/deploy_gh_gl#github-actions-setup"
										target="_blank"
										class="text-blue-500 hover:underline font-mono">open-pr-on-commit</a
									>
									workflow in the repository. Recommended: connect the repository through the
									<a
										href="https://www.windmill.dev/docs/integrations/git_repository#github-app"
										target="_blank"
										class="text-blue-500 hover:underline">GitHub App</a
									> and Windmill opens them automatically.
								</div>
							{/if}
							{#if repoMode === 'sync' && isFork}
								<div class="text-2xs text-secondary mt-2">
									Deploys from this workspace are pushed to the
									<span class="font-mono">{forkBranch}</span> branch of the shared repository, not to
									the tracked branch.
								</div>
							{:else if repoMode === 'sync' && !isFork}
								<div class="text-2xs text-secondary mt-2">
									These push settings also apply to forks of this workspace: an item deployed in a
									fork is pushed to the fork's own
									<span class="font-mono">wm-fork/…</span> branch instead of the tracked branch.
								</div>
								{#if isGithubApp}
									<div class="mt-2">
										<Toggle
											checked={repo.fork_open_prs ?? false}
											disabled={!$enterpriseLicense}
											options={{
												right: 'Open a pull request when an item is deployed in a fork',
												rightTooltip:
													"After an item deployed in a fork is pushed to the fork's branch (wm-fork/**, or the dev branch for a dev workspace), Windmill opens a pull request to the tracked branch of the shared repository. Runs from the deploy itself, so it works without inbound webhooks. When a dev workspace enables Git promotion, its own pull request toggle takes over for its wm_deploy/** branches."
											}}
											on:change={(e) => setForkOpenPrs(e.detail)}
										>
											{#snippet right()}
												{#if !$enterpriseLicense}<EEOnly />{/if}
											{/snippet}
										</Toggle>
									</div>
								{:else}
									<div class="text-2xs text-secondary mt-2">
										To open pull requests when an item is deployed in a fork, set up the
										<a
											href="https://www.windmill.dev/docs/advanced/git_sync#github-actions"
											target="_blank"
											class="text-blue-500 hover:underline font-mono">open-pr-on-fork-commit</a
										>
										workflow in the repository. Recommended: connect the repository through the
										<a
											href="https://www.windmill.dev/docs/integrations/git_repository#github-app"
											target="_blank"
											class="text-blue-500 hover:underline">GitHub App</a
										> and Windmill opens them automatically.
									</div>
								{/if}
							{/if}
							{#if repo.open_pr_error}
								<div class="mt-2">
									<Alert type="warning" title="Last pull request couldn't be opened" size="xs">
										{repo.open_pr_error} If this mentions permissions, the GitHub App installation may
										not have approved the pull-request permission yet.
									</Alert>
								</div>
							{/if}
						</div>

						<!-- Direction 2: Git -> Windmill (pull / auto-deploy). Promotion repos
						     push deploy branches on top of a sync-mode setup; the pull
						     direction belongs to that sync repo, so it's hidden here. -->
						{#if repoMode === 'sync'}
							<div class="mt-4 border-t border-gray-200 pt-3 dark:border-gray-700">
								<div class="flex justify-between items-start gap-4">
									<div class="text-sm font-semibold text-emphasis"
										>Pull from Git (Git → Windmill)</div
									>
									<Button
										size="xs"
										variant="default"
										onclick={() => idx !== null && gitSyncContext.showPullModal(idx)}
										startIcon={{ icon: Download }}
									>
										Pull from repo
									</Button>
								</div>
								{#if isFork}
									<!-- Fork sync is parent-managed: no control here, only status. -->
									<div class="text-2xs text-secondary mt-2">
										Automatic sync from Git is managed in the parent workspace's git sync settings.
										When enabled there, changes to this fork's
										<span class="font-mono">{forkBranch}</span> branch deploy here automatically.
									</div>
									{#if repo.auto_pull?.last_pull_status}
										<div class="text-2xs text-secondary mt-2">
											{#if repo.auto_pull.last_pull_status.success}
												Last synced{repo.auto_pull.last_pull_status.synced_sha
													? ` to ${repo.auto_pull.last_pull_status.synced_sha.slice(0, 7)}`
													: ''}.
											{:else}
												<span class="text-red-600 dark:text-red-400">
													Last sync failed{repo.auto_pull.last_pull_status.error
														? `: ${repo.auto_pull.last_pull_status.error}`
														: ''}.
												</span>
											{/if}
										</div>
									{/if}
								{:else}
									<Toggle
										checked={repo.auto_pull?.enabled ?? false}
										disabled={!$enterpriseLicense}
										options={{
											right: 'Automatically deploy changes from Git',
											rightTooltip:
												'Windmill deploys new commits from the tracked branch into this workspace. Repositories connected through the GitHub App sync instantly via webhooks with a polling fallback; token-based repositories are checked about every minute.'
										}}
										on:change={(e) => setAutoPullEnabled(e.detail)}
									>
										{#snippet right()}
											{#if !$enterpriseLicense}<EEOnly />{/if}
										{/snippet}
									</Toggle>
									<div class="mt-1">
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
									</div>
								{/if}
								{#if !isGithubApp && !loadingResourceInfo}
									<div class="mt-2">
										<Alert type="info" title="Instant pull recommended" size="xs">
											Pull for this repository checks the tracked branch about every minute; longer
											gaps make drift and merge conflicts more likely. For instant pull, connect the
											repository through the
											<a
												href="https://www.windmill.dev/docs/integrations/git_repository#github-app"
												target="_blank"
												class="text-blue-500 hover:underline">GitHub App</a
											>
											(which also lets Windmill manage pull requests), or push changes into Windmill
											with the
											<a
												href="https://www.windmill.dev/docs/advanced/git_sync#github-actions"
												target="_blank"
												class="text-blue-500 hover:underline">sync GitHub workflow</a
											>. If you already push changes with a GitHub Action, keep either the Action or
											automatic pull, not both, so they don't fight over deploys.
										</Alert>
									</div>
								{/if}
								{#if repo.auto_pull?.enabled}
									{@const viaWebhook = repo.auto_pull?.webhook_id != null}
									{#if isGithubApp}
										<div class="mt-2">
											<Alert type="info" title="Already pulling with a GitHub Action?" size="xs">
												If you previously set up a GitHub Action to push changes into Windmill,
												remove it now so the two don't fight over deploys.
											</Alert>
										</div>
									{/if}
									<div class="text-2xs text-secondary mt-2">
										{#if repo.auto_pull?.last_pull_status}
											{#if repo.auto_pull.last_pull_status.success}
												Last synced{repo.auto_pull.last_pull_status.synced_sha
													? ` to ${repo.auto_pull.last_pull_status.synced_sha.slice(0, 7)}`
													: ''}.
												{viaWebhook
													? ' Syncing instantly via webhook.'
													: ' Checking the tracked branch about every minute.'}
											{:else}
												<span class="text-red-600 dark:text-red-400">
													Last sync failed{repo.auto_pull.last_pull_status.error
														? `: ${repo.auto_pull.last_pull_status.error}`
														: ''}.
												</span>
											{/if}
										{:else}
											{viaWebhook
												? 'Connected via webhook. New commits to the tracked branch deploy instantly.'
												: 'Checking the tracked branch about every minute. New commits deploy automatically.'}
										{/if}
									</div>
									{#if isGithubApp && repo.auto_pull?.webhook_error}
										<div class="mt-2">
											<Alert type="warning" title="Falling back to polling" size="xs">
												{repo.auto_pull.webhook_error}
											</Alert>
										</div>
									{/if}
								{/if}
							</div>
						{/if}
					{/if}
				{/if}
			{/if}
		{:else}
			<div class="text-xs text-primary">Please select a Git repository resource.</div>
		{/if}
	</div>
{/snippet}

<!-- Main component rendering -->

{#if shouldShowEmptyState}
	<!-- Empty State for Primary Variants -->
	<div class="rounded-md shadow-sm bg-surface-tertiary p-4 mb-4">
		<div class="flex items-center justify-between mb-4">
			<div class="flex flex-col">
				<h3 class="text-xs font-semibold text-emphasis">{displayTitle}</h3>
				{#if displayDescription}
					<p class="text-2xs text-secondary"
						>{displayDescription}
						{#if mode === 'promotion'}
							<a target="_blank" href="https://www.windmill.dev/docs/advanced/deploy_gh_gl"
								>Learn more about Git Promotion</a
							>
						{/if}
					</p>
				{/if}
			</div>
		</div>

		<div
			class="flex flex-col items-center justify-center py-8 px-4 border-2 border-dashed rounded-md border-border-normal"
		>
			<div class="text-center mb-4">
				<p class="text-primary text-xs font-normal mb-2">
					{#if mode === 'sync'}
						No Git Sync repository configured. Add one to enable direct synchronization.
					{:else if mode === 'promotion'}
						No Git Promotion repository configured. Add one to enable branch-based workflows.
					{:else}
						No repository configured.
					{/if}
				</p>
			</div>
			{#if onAdd}
				<Button unifiedSize="md" variant="default" startIcon={{ icon: Plus }} onclick={onAdd}>
					Add {mode || 'repository'} repository
				</Button>
			{/if}
		</div>
	</div>
{:else if repo}
	{#if variant === 'primary-sync' || variant === 'primary-promotion'}
		<!-- Primary Repository Layout -->
		<div class="rounded-md shadow-sm bg-surface-tertiary p-4 mb-4">
			<div class="flex flex-col mb-4 gap-2">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold">{displayTitle}</h3>

					<div class="flex items-center gap-2">
						{@render headerActions()}
					</div>
				</div>
				{#if displayDescription}
					<p class="text-xs text-secondary"
						>{displayDescription}
						{#if mode === 'promotion'}
							<a target="_blank" href="https://www.windmill.dev/docs/advanced/deploy_gh_gl"
								>Learn more about Git Promotion</a
							>
						{/if}</p
					>
				{/if}
			</div>
			{@render repositoryContent()}
		</div>
	{:else}
		<!-- Standard Repository Card Layout -->
		<div class="rounded-md shadow-sm bg-surface-tertiary p-0 w-full mb-4">
			<div class="flex items-center justify-between min-h-10 px-4 py-1 border-b">
				<div class="flex items-center gap-2">
					<span class="text-lg font-semibold">{displayTitle}</span>
					{#if repo.legacyImported}
						<span
							class="inline-flex items-center px-2 py-1 rounded-full text-xs font-semibold bg-orange-100 text-orange-800 border border-orange-200"
						>
							Legacy Configuration
						</span>
					{/if}
					<span class="text-xs text-primary pt-1 pl-8">
						{repo.git_repo_resource_path}
					</span>
				</div>
				<div class="flex items-center gap-2">
					{@render headerActions()}
				</div>
			</div>
			{#if !repo.collapsed}
				<div class="px-4 py-2">
					{@render repositoryContent()}
				</div>
			{/if}
		</div>
	{/if}
{/if}
