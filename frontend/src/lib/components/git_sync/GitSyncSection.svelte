<script lang="ts">
	import { ExternalLink, ChevronDown, ChevronRight, Plus } from 'lucide-svelte'
	import { Button, Alert } from '$lib/components/common'
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
	const devPrimaryRepo = $derived(isDevWorkspace ? (primarySync ?? primaryPromotion) : null)
	// The single-repo dev UX (one card + promotion toggle, secondaries hidden) is
	// only safe when the dev actually has one repo — i.e. it inherited prod's on
	// fork. An ATTACHED dev keeps its own repos: with more than one, fall back to
	// the normal layout so none are hidden and we don't present an unrelated repo
	// as prod's promotion target.
	const devSingleRepo = $derived(isDevWorkspace && (gitSyncContext?.repositories?.length ?? 0) <= 1)
	const secondarySync = $derived(gitSyncContext?.getSecondarySyncRepositories() || [])
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

	// State for collapsible sections
	let secondarySyncExpanded = $state(false)
	let secondaryPromotionExpanded = $state(false)

	// Check if any secondary repositories are unsaved
	const hasUnsavedSecondary = $derived(secondarySync.some((s) => s.repo.isUnsavedConnection))
	const hasUnsavedSecondaryPromotion = $derived(
		secondaryPromotion.some((s) => s.repo.isUnsavedConnection)
	)
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
					variant="accent"
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
		<!-- Primary Sync Repository -->
		<div class="space-y-6 pt-6">
			<GitSyncRepositoryCard
				variant="primary-sync"
				mode={devSingleRepo && devPrimaryRepo?.repo?.use_individual_branch ? 'promotion' : 'sync'}
				idx={(devSingleRepo ? devPrimaryRepo : primarySync)?.idx ?? null}
				repository={(devSingleRepo ? devPrimaryRepo : primarySync)?.repo ?? null}
				onAdd={() => gitSyncContext.addSyncRepository()}
				isCollapsible={false}
				showEmptyState={(devSingleRepo ? devPrimaryRepo : primarySync)?.repo == null}
				devPromotion={devSingleRepo && !!$enterpriseLicense}
			/>

			{#if $enterpriseLicense}
				<!-- Secondary Sync Repositories (EE only; a dev workspace has a single inherited repo) -->
				{#if primarySync && !primarySync.repo?.isUnsavedConnection && !devSingleRepo}
					{#if secondarySync.length > 0 || secondarySyncExpanded}
						<div class="mt-4">
							<button
								class="flex items-center gap-2 text-sm text-secondary hover:text-primary transition-colors"
								onclick={() => (secondarySyncExpanded = !secondarySyncExpanded)}
							>
								{#if secondarySyncExpanded}
									<ChevronDown size={16} />
								{:else}
									<ChevronRight size={16} />
								{/if}
								Secondary sync repositories ({secondarySync.length})
							</button>

							{#if secondarySyncExpanded}
								<div class="mt-3 space-y-3">
									{#if secondarySync.length === 0}
										<div class="text-sm text-secondary italic">
											No secondary sync repositories configured
										</div>
									{:else}
										{#each secondarySync as { repo, idx } (repo.git_repo_resource_path)}
											<div class="pl-4">
												<GitSyncRepositoryCard variant="secondary" {idx} isSecondary={true} />
											</div>
										{/each}
									{/if}

									{#if !hasUnsavedSecondary}
										<div class="pl-4">
											<Button
												size="xs"
												variant="default"
												startIcon={{ icon: Plus }}
												onclick={() => gitSyncContext.addSyncRepository()}
											>
												Add secondary sync
											</Button>
										</div>
									{/if}
								</div>
							{/if}
						</div>
					{:else}
						<!-- Collapsed state when no secondary repos exist -->
						{#if !hasUnsavedSecondary}
							<div class="mt-2">
								<button
									class="text-xs text-primary hover:text-secondary transition-colors"
									onclick={() => {
										secondarySyncExpanded = true
										gitSyncContext.addSyncRepository()
									}}
								>
									+ Add secondary sync repository
								</button>
							</div>
						{/if}
					{/if}
				{/if}

				<!-- Primary Promotion Repository (EE only; roots only — a dev promotes via the
					toggle on its single inherited repo, not a separate promotion repo) -->
				{#if showPromotion && !devSingleRepo}
					<div class="mt-6">
						<GitSyncRepositoryCard
							variant="primary-promotion"
							mode="promotion"
							idx={primaryPromotion?.idx ?? null}
							repository={primaryPromotion?.repo ?? null}
							onAdd={() => gitSyncContext.addPromotionRepository()}
							isCollapsible={false}
							showEmptyState={primaryPromotion?.repo === null}
						/>

						<!-- Secondary Promotion Repositories -->
						{#if primaryPromotion && !primaryPromotion.repo?.isUnsavedConnection}
							{#if secondaryPromotion.length > 0 || secondaryPromotionExpanded}
								<div class="mt-4">
									<button
										class="flex items-center gap-2 text-sm text-secondary hover:text-primary transition-colors"
										onclick={() => (secondaryPromotionExpanded = !secondaryPromotionExpanded)}
									>
										{#if secondaryPromotionExpanded}
											<ChevronDown size={16} />
										{:else}
											<ChevronRight size={16} />
										{/if}
										Secondary promotion repositories ({secondaryPromotion.length})
									</button>

									{#if secondaryPromotionExpanded}
										<div class="mt-3 space-y-3">
											{#if secondaryPromotion.length === 0}
												<div class="text-sm text-secondary italic">
													No secondary promotion repositories configured
												</div>
											{:else}
												{#each secondaryPromotion as { repo, idx } (repo.git_repo_resource_path)}
													<div class="pl-4">
														<GitSyncRepositoryCard variant="secondary" {idx} isSecondary={true} />
													</div>
												{/each}
											{/if}

											{#if !hasUnsavedSecondaryPromotion}
												<div class="pl-4">
													<Button
														size="xs"
														variant="default"
														startIcon={{ icon: Plus }}
														onclick={() => gitSyncContext.addPromotionRepository()}
													>
														Add secondary promotion
													</Button>
												</div>
											{/if}
										</div>
									{/if}
								</div>
							{:else}
								<!-- Collapsed state when no secondary promotion repos exist -->
								{#if !hasUnsavedSecondaryPromotion}
									<div class="mt-2">
										<button
											class="text-xs text-primary hover:text-secondary transition-colors"
											onclick={() => {
												secondaryPromotionExpanded = true
												gitSyncContext.addPromotionRepository()
											}}
										>
											+ Add secondary promotion repository
										</button>
									</div>
								{/if}
							{/if}
						{/if}
					</div>
				{:else if !showPromotion}
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

		<!-- Modals -->
		<GitSyncModalManager />
	{/if}
{/if}
