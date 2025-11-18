<script lang="ts">
	import { ExternalLink, ChevronDown, ChevronRight, Plus } from 'lucide-svelte'
	import { Button, Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { setGitSyncContext } from './GitSyncContext.svelte'
	import GitSyncRepositoryCard from './GitSyncRepositoryCard.svelte'
	import GitSyncModalManager from './GitSyncModalManager.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { untrack } from 'svelte'

	// Create context reactively based on workspaceStore
	const gitSyncContext = $derived($workspaceStore ? setGitSyncContext($workspaceStore) : null)

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
	const secondarySync = $derived(gitSyncContext?.getSecondarySyncRepositories() || [])
	const secondaryPromotion = $derived(gitSyncContext?.getSecondaryPromotionRepositories() || [])

	// State for collapsible sections
	let secondarySyncExpanded = $state(false)
	let secondaryPromotionExpanded = $state(false)

	// Check if any secondary repositories are unsaved
	const hasUnsavedSecondary = $derived(secondarySync.some((s) => s.repo.isUnsavedConnection))
	const hasUnsavedSecondaryPromotion = $derived(secondaryPromotion.some((s) => s.repo.isUnsavedConnection))
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
	<div class="flex flex-col gap-4 my-8">
		<div class="flex flex-col gap-1">
			<div class="text-emphasis text-sm font-semibold">Git Sync</div>
			<Description link="https://www.windmill.dev/docs/advanced/git_sync">
				Connect the Windmill workspace to a Git repository to automatically commit and push scripts,
				flows, and apps to the repository on each deploy.
			</Description>
		</div>
		<Alert type="info" title="Only new updates trigger git sync">
			Only new changes matching the filters will trigger a git sync. You still need to initialize
			the repo to the desired state first.
		</Alert>
	</div>
	{#if !$enterpriseLicense}
		<div class="mb-2"></div>

		<Alert type="warning" title="Syncing workspace to Git is an EE feature">
			Automatically saving scripts to a Git repository on each deploy is a Windmill EE feature.
		</Alert>
		<div class="mb-2"></div>
	{/if}
	{#if $enterpriseLicense && gitSyncContext.repositories != undefined}
		<div class="flex mt-5 mb-5 gap-8">
			<Button
				variant="accent"
				target="_blank"
				endIcon={{ icon: ExternalLink }}
				href={`/runs?job_kinds=deploymentcallbacks&workspace=${$workspaceStore}`}
			>
				See sync jobs
			</Button>
		</div>
		<div class="pt-2"></div>

		<!-- Primary Sync Repository -->
		<div class="space-y-4">
			<GitSyncRepositoryCard
				variant="primary-sync"
				mode="sync"
				idx={primarySync?.idx ?? null}
				repository={primarySync?.repo ?? null}
				onAdd={() => gitSyncContext.addSyncRepository()}
				isCollapsible={false}
				showEmptyState={primarySync?.repo === null}
			/>

			<!-- Secondary Sync Repositories -->
			{#if primarySync && !primarySync.repo?.isUnsavedConnection}
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

			<!-- Primary Promotion Repository -->
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
		</div>

		<!-- Modals -->
		<GitSyncModalManager />
	{/if}
{/if}
