<script lang="ts">
	import { ExternalLink, Plus, Settings, GitBranch } from 'lucide-svelte'
	import { Button, Alert, Badge, Drawer, DrawerContent } from '$lib/components/common'
	import GitSyncSetupModal from './GitSyncSetupModal.svelte'
	import EEOnly from '$lib/components/EEOnly.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import { setGitSyncContext } from './GitSyncContext.svelte'
	import GitSyncRepositoryCard from './GitSyncRepositoryCard.svelte'
	import GitSyncModalManager from './GitSyncModalManager.svelte'
	import { enterpriseLicense, workspaceStore, userWorkspaces } from '$lib/stores'
	import { base } from '$lib/base'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { untrack } from 'svelte'

	// Create context reactively based on workspaceStore
	const gitSyncContext = $derived($workspaceStore ? setGitSyncContext($workspaceStore) : null)

	// Fetch git sync eligibility
	let gitSyncStatus = $state<{
		enabled: boolean
		reason: string | null
		max_repos: number | null
		user_count: number | null
		max_users: number | null
	}>({ enabled: false, reason: null, max_repos: null, user_count: null, max_users: null })

	$effect(() => {
		if ($workspaceStore) {
			WorkspaceService.getGitSyncEnabled({ workspace: $workspaceStore })
				.then((status) => {
					gitSyncStatus = status as typeof gitSyncStatus
				})
				.catch(() => {
					gitSyncStatus = {
						enabled: false,
						reason: null,
						max_repos: null,
						user_count: null,
						max_users: null
					}
				})
		}
	})

	const gitSyncAllowed = $derived(gitSyncStatus.enabled)
	const isFreeTier = $derived(gitSyncAllowed && !$enterpriseLicense)
	// Throwaway forks never run promotion mode: their deploys always go to the
	// fork's own wm-fork/** branch, so a promotion repo could never take effect
	// (the backend rejects it too). A dev workspace is the exception — it deploys
	// per-item wm_deploy/** branches that promote into its parent. Mirrors the
	// backend/CLI rule.
	const currentWorkspace = $derived($userWorkspaces?.find((w) => w.id === $workspaceStore))
	const isFork = $derived(
		($workspaceStore?.startsWith('wm-fork-') ?? false) || !!currentWorkspace?.parent_workspace_id
	)
	const isDevWorkspace = $derived(!!currentWorkspace?.is_dev_workspace)
	const showPromotion = $derived(!isFork || isDevWorkspace)
	const hasConfiguredRepos = $derived(
		gitSyncContext?.repositories?.some((r) => r.git_repo_resource_path) ?? false
	)

	// Load settings when workspace context changes
	$effect(() => {
		if (gitSyncContext) {
			untrack(async () => {
				try {
					await gitSyncContext.loadSettings()
				} catch (error) {
					console.error('Failed to load git sync settings:', error)
					sendUserToast('Failed to load git sync settings', true)
				}
			})
		}
	})

	// Derived state for repository categorization
	const primarySync = $derived(gitSyncContext?.getPrimarySyncRepository() || null)
	const primaryPromotion = $derived(gitSyncContext?.getPrimaryPromotionRepository() || null)
	// A dev workspace reuses the single repo it inherited from prod: whether it's
	// currently in sync or promotion mode, it's the same one card, toggled between
	// the two — so a dev never configures a separate promotion repo.
	// The single-repo dev UX (one card + promotion toggle, secondaries hidden) is
	// only safe when the dev actually has one repo — i.e. it inherited prod's on
	// fork. An ATTACHED dev keeps its own repos: with more than one, fall back to
	// the normal layout so none are hidden and we don't present an unrelated repo
	// as prod's promotion target.
	const devSingleRepo = $derived(isDevWorkspace && (gitSyncContext?.repositories?.length ?? 0) <= 1)
	const secondaryPromotion = $derived(gitSyncContext?.getSecondaryPromotionRepositories() || [])
	// Fork creation keeps only sync-mode repositories and a fork is refused a
	// promotion one, so the single way a fork holds one is a dev workspace
	// detached back into a plain fork. Deploys still sync through it (only the
	// promotion branching is dropped), so name it instead of showing nothing.
	const promotionModeRepos = $derived(
		showPromotion ? [] : [primaryPromotion, ...secondaryPromotion].filter((r) => r != null)
	)
	// Promotion is what a dev workspace does, so a fork that wants it can be
	// re-designated as one. Pairing is prod-scoped and admin-gated there, so this
	// only links to the parent's screen, and only when the parent is a workspace
	// the user actually has.
	const devPairingHref = $derived.by(() => {
		const parent = currentWorkspace?.parent_workspace_id
		if (!parent || !$userWorkspaces?.some((w) => w.id === parent)) return undefined
		return `${base}/workspace_settings?workspace=${parent}&tab=dev_workspace`
	})

	const repositories = $derived(gitSyncContext?.repositories ?? [])
	const hasUnsavedConnection = $derived(repositories.some((r) => r.isUnsavedConnection))
	const usedResourcePaths = $derived(
		repositories.map((r) => r.git_repo_resource_path).filter((p) => !!p?.trim())
	)

	let setupMode = $state<'sync' | 'promotion'>('sync')
	let setupOpen = $state(false)
	function openSetup(mode: 'sync' | 'promotion') {
		setupMode = mode
		setupOpen = true
	}
	const setupTitle = $derived(
		setupMode === 'promotion'
			? 'Add a promotion repository'
			: repositories.length === 0
				? 'Configure Git Sync'
				: 'Add a secondary sync repository'
	)

	// Keyed by resource path rather than index: deleting a repository shifts the
	// indexes, and reloading the settings replaces the objects.
	let settingsPath: string | undefined = $state(undefined)
	const settingsIdx = $derived(
		settingsPath === undefined
			? -1
			: repositories.findIndex((r) => r.git_repo_resource_path === settingsPath)
	)
	let settingsDrawer: Drawer | undefined = $state(undefined)
	$effect(() => {
		if (settingsPath !== undefined && settingsIdx === -1)
			untrack(() => settingsDrawer?.closeDrawer())
	})
	function openSettings(path: string) {
		settingsPath = path
		settingsDrawer?.openDrawer()
	}

	function addWithExistingResource(path: string) {
		if (!gitSyncContext) return
		const before = gitSyncContext.repositories.length
		if (setupMode === 'promotion') gitSyncContext.addPromotionRepository()
		else gitSyncContext.addSyncRepository()
		if (gitSyncContext.repositories.length === before) return
		gitSyncContext.repositories[before].git_repo_resource_path = path
		openSettings(path)
	}

	function cardProps(idx: number) {
		const repo = repositories[idx]
		if (devSingleRepo) {
			return {
				variant: 'primary-sync' as const,
				mode: repo?.use_individual_branch ? ('promotion' as const) : ('sync' as const),
				devPromotion: !!$enterpriseLicense
			}
		}
		if (idx === primarySync?.idx) return { variant: 'primary-sync' as const, mode: 'sync' as const }
		if (idx === primaryPromotion?.idx)
			return { variant: 'primary-promotion' as const, mode: 'promotion' as const }
		return { variant: 'secondary' as const, isSecondary: true }
	}

	function rowLabel(idx: number): string {
		const repo = repositories[idx]
		const promotion = !!repo?.use_individual_branch
		if (devSingleRepo) return promotion ? 'Git Promotion' : 'Git Sync'
		const primary = idx === primarySync?.idx || idx === primaryPromotion?.idx
		return `${primary ? 'Primary' : 'Secondary'} ${promotion ? 'promotion' : 'sync'} repository`
	}

	// Shown without EE too, disabled, so CE users see what the upgrade unlocks.
	const showAddSecondarySync = $derived(
		!devSingleRepo &&
			!!primarySync &&
			!primarySync.repo.isUnsavedConnection &&
			!hasUnsavedConnection
	)
	const showAddPromotion = $derived(showPromotion && !devSingleRepo && !hasUnsavedConnection)
</script>

{#if !gitSyncContext}
	<div class="flex items-center justify-center p-8">
		<div class="text-sm text-secondary">Loading workspace...</div>
	</div>
{:else if gitSyncContext.loading}
	<div class="flex items-center justify-center p-8">
		<div class="text-sm text-secondary">Loading git sync settings...</div>
	</div>
{:else}
	<SettingsPageHeader
		title="Git Sync"
		description="Connect the Windmill workspace to a Git repository: each deploy commits scripts, flows, and apps to the repository, and new commits to the repository can automatically deploy into the workspace."
		link="https://www.windmill.dev/docs/advanced/git_sync"
	>
		{#snippet actions()}
			{#if (gitSyncAllowed || gitSyncStatus.user_count != null) && gitSyncContext?.repositories != undefined}
				<Button
					variant={repositories.length > 0 ? 'accent' : 'default'}
					target="_blank"
					endIcon={{ icon: ExternalLink }}
					href={`/runs?job_kinds=deploymentcallbacks&workspace=${$workspaceStore}`}
				>
					See sync jobs
				</Button>
			{/if}
		{/snippet}
	</SettingsPageHeader>
	<Alert type="info" title="Only new updates trigger git sync">
		Only new changes matching the filters will trigger a git sync. You still need to initialize the
		repo to the desired state first.
	</Alert>
	{#if !gitSyncAllowed}
		<div class="mb-2"></div>

		<Alert type={hasConfiguredRepos ? 'error' : 'warning'} title="Git sync disabled">
			Git sync is an EE feature provided in CE for testing and hobbyist use when workspace members
			&le; {gitSyncStatus.max_users}. Your workspace has {gitSyncStatus.user_count} members. Settings
			below are preserved but sync is inactive until membership is reduced or you upgrade to EE.
		</Alert>
		<div class="mb-2"></div>
	{:else if isFreeTier}
		<div class="mb-2"></div>

		<Alert type="warning" title="CE Limited Feature">
			Git sync is an EE feature provided in CE for testing and hobbyist use when workspace members
			&le; {gitSyncStatus.max_users}. Limited to a single repository. Upgrade to EE for multiple
			repositories, promotion mode, and GitHub App authentication.
		</Alert>
		<div class="mb-2"></div>
	{/if}
	{#if (gitSyncAllowed || gitSyncStatus.user_count != null) && gitSyncContext?.repositories != undefined}
		<div class="pt-6">
			{#if repositories.length === 0}
				<div
					class="flex flex-col items-center gap-3 text-center py-10 px-4 border rounded-md bg-surface-tertiary"
				>
					<GitBranch size={24} class="text-secondary" />
					<div class="flex flex-col gap-1 items-center">
						<span class="font-semibold text-sm text-emphasis">No repository connected</span>
						<p class="text-xs text-secondary max-w-sm">
							Commit every deploy of this workspace to a Git repository, and deploy new commits back
							into it.
						</p>
					</div>
					<Button unifiedSize="md" variant="accent" onClick={() => openSetup('sync')}>
						Configure Git Sync
					</Button>
				</div>
			{:else}
				<div class="flex flex-col border rounded-md divide-y bg-surface-tertiary">
					{#each repositories as repo, idx (idx)}
						{@const validation = gitSyncContext.getValidation(idx)}
						<div class="flex items-center justify-between gap-4 px-4 py-3">
							<div class="flex flex-col gap-0.5 min-w-0">
								<div class="flex items-center gap-2 min-w-0">
									<span class="text-xs font-medium text-emphasis truncate">
										{repo.git_repo_resource_path || 'No resource selected'}
									</span>
									{#if repo.isUnsavedConnection}
										<Badge small color="yellow">Not saved</Badge>
									{:else if validation?.hasChanges}
										<Badge small color="yellow">Unsaved changes</Badge>
									{/if}
									{#if repo.legacyImported}
										<Badge small color="orange">Legacy configuration</Badge>
									{/if}
								</div>
								<span class="text-2xs text-secondary">{rowLabel(idx)}</span>
							</div>
							<Button
								unifiedSize="sm"
								variant="default"
								startIcon={{ icon: Settings }}
								onClick={() => openSettings(repo.git_repo_resource_path)}
							>
								Settings
							</Button>
						</div>
					{/each}
				</div>

				{#if showAddSecondarySync || showAddPromotion}
					<div class="flex gap-4 mt-3">
						{#if showAddSecondarySync}
							<div class="flex items-center gap-1">
								<Button
									unifiedSize="sm"
									variant="default"
									startIcon={{ icon: Plus }}
									disabled={!$enterpriseLicense}
									onClick={() => openSetup('sync')}
								>
									Add secondary sync repository
								</Button>
								{#if !$enterpriseLicense}<EEOnly />{/if}
							</div>
						{/if}
						{#if showAddPromotion}
							<div class="flex items-center gap-1">
								<Button
									unifiedSize="sm"
									variant="default"
									startIcon={{ icon: Plus }}
									disabled={!$enterpriseLicense}
									onClick={() => openSetup('promotion')}
								>
									Add promotion repository
								</Button>
								{#if !$enterpriseLicense}<EEOnly />{/if}
							</div>
						{/if}
					</div>
				{/if}

				{#if $enterpriseLicense && !showPromotion}
					<div class="mt-6">
						<Alert
							type="info"
							title="Promotion does not apply to a fork"
							documentationLink="https://www.windmill.dev/docs/advanced/workspace_forks"
						>
							Deploys in a fork always commit to the fork's own wm-fork/** branch, so a promotion
							repository would never take effect here. Promote this fork's work by merging that
							branch into the tracked branch instead.
							{#if devPairingHref}
								<div class="mt-2">
									To promote per item from this workspace, pair it with its parent as a
									<a href={devPairingHref} class="text-blue-500 hover:underline">dev workspace</a>.
								</div>
							{/if}
							{#if promotionModeRepos.length > 0}
								<div class="mt-2">
									Still set to promotion mode here, and still syncing deploys to the fork's branch:
									{promotionModeRepos.map((r) => r.repo.git_repo_resource_path).join(', ')}
								</div>
							{/if}
						</Alert>
					</div>
				{/if}
			{/if}
		</div>

		<GitSyncSetupModal
			bind:opened={setupOpen}
			title={setupTitle}
			{usedResourcePaths}
			onExistingResource={addWithExistingResource}
		/>

		<Drawer
			bind:this={settingsDrawer}
			size="1000px"
			on:afterClose={() => (settingsPath = undefined)}
		>
			<DrawerContent
				title={settingsIdx !== -1 ? rowLabel(settingsIdx) : 'Repository settings'}
				on:close={() => settingsDrawer?.closeDrawer()}
			>
				{#if settingsIdx !== -1}
					{#key settingsIdx}
						<GitSyncRepositoryCard
							idx={settingsIdx}
							isCollapsible={false}
							{...cardProps(settingsIdx)}
						/>
					{/key}
				{/if}
			</DrawerContent>
		</Drawer>

		<!-- Modals -->
		<GitSyncModalManager />
	{/if}
{/if}
