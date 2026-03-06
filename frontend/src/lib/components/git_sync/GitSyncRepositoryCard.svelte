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
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Section from '$lib/components/Section.svelte'
	import { getGitSyncContext } from './GitSyncContext.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import GitSyncFilterSettings from '$lib/components/workspaceSettings/GitSyncFilterSettings.svelte'
	import DetectionFlow from './DetectionFlow.svelte'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '$lib/components/Toggle.svelte'
	import { fade } from 'svelte/transition'
	import { workspaceStore } from '$lib/stores'
	import hubPaths from '$lib/hubPaths.json'
	import type { GitSyncRepository } from './GitSyncContext.svelte'
	import GitSyncModeDisplay from './GitSyncModeDisplay.svelte'
	import { DEFAULT_HUB_BASE_URL } from '$lib/hub'
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
		showEmptyState = false
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
	}>()

	const gitSyncContext = getGitSyncContext()
	const repo = $derived(repository || (idx !== null ? gitSyncContext.getRepository(idx) : null))
	const validation = $derived(idx !== null ? gitSyncContext.getValidation(idx) : null)
	const gitSyncTestJob = $derived(idx !== null ? gitSyncContext.gitSyncTestJobs?.[idx] : null)
	let confirmingDelete = $state(false)
	let targetBranch = $state<string | undefined>(undefined) // Default to main, will be updated when resource is available
	let resourceInfo = $state<{ url?: string; error?: string } | null>(null)
	let loadingResourceInfo = $state(false)

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
			if (repo?.git_repo_resource_path && !repo.isUnsavedConnection && $workspaceStore) {
				loadingResourceInfo = true
				resourceInfo = null
				try {
					const resource = await ResourceService.getResource({
						workspace: $workspaceStore,
						path: repo.git_repo_resource_path
					})

					if (!abortController.signal.aborted && resource?.value) {
						// Extract git URL from resource value
						const value = resource.value as Record<string, any>
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
				? `Changes will be committed directly to the ${targetOrDefaultBranch} branch`
				: mode === 'promotion'
					? `Changes will be made to new branches whose promotion target is the ${targetOrDefaultBranch} branch of the repo to promote to. Remember to also setup Git Sync between the promotion workspace and repo for changes to be deployed when these branches are merged.`
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
					disabled={emptyString(repo.script_path)}
					variant="accent"
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

			{#if repo.script_path != hubPaths.gitSync}
				<Alert type="warning" title="Script version mismatch">
					The git sync version for this repository is not latest. Current: <a
						target="_blank"
						href="{DEFAULT_HUB_BASE_URL}/scripts/windmill/6943/sync-script-to-git-repo-windmill/9014/versions"
						>{repo.script_path}</a
					>, latest:
					<a
						target="_blank"
						href="{DEFAULT_HUB_BASE_URL}/scripts/windmill/6943/sync-script-to-git-repo-windmill/9014/versions"
						>{hubPaths.gitSync}</a
					>
					<div class="flex mt-2">
						<Button
							size="xs"
							variant="accent"
							onclick={() => {
								if (repo) {
									repo.script_path = hubPaths.gitSync
								}
							}}
						>
							Update git sync script (require save git settings to be applied)
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
					<div class="flex justify-between items-start">
						<!-- Display mode settings as prominent text -->
						<div class="flex-1 mr-4">
							<GitSyncModeDisplay mode={repoMode} {targetBranch} repository={repo} />
						</div>

						<!-- Manual sync section for existing repos -->
						{#if !emptyString(repo.git_repo_resource_path) && !repo.legacyImported}
							<div class="flex flex-col">
								<div class="text-sm text-secondary mb-2">Manual workspace content sync</div>
								<div class="flex gap-2">
									<Button
										size="xs"
										variant="default"
										onclick={() => idx !== null && gitSyncContext.showPullModal(idx)}
										startIcon={{ icon: Download }}
									>
										Pull from repo
									</Button>
									<Button
										size="xs"
										variant="default"
										onclick={() => idx !== null && gitSyncContext.showPushModal(idx)}
										startIcon={{ icon: Upload }}
									>
										Push to repo
									</Button>
								</div>
							</div>
						{/if}
					</div>

					<!-- Advanced settings (collapsible) -->
					<Section label="Advanced" small collapsable initiallyCollapsed={!repo.force_branch}>
						<Toggle
							checked={!!repo.force_branch}
							on:change={(e) => {
								if (e.detail) {
									repo.force_branch = $workspaceStore ?? ''
								} else {
									repo.force_branch = undefined
								}
							}}
							options={{
								right: 'Environment (experimental)',
								rightTooltip:
									'Made for monobranch setups. Passes the value as --branch/--env to the wmill CLI, which selects the matching branch/env configuration from wmill.yaml and includes the branch/env in the item paths.'
							}}
						/>
						{#if repo.force_branch != null && repo.force_branch !== undefined}
							<div class="w-48 mt-2">
								<TextInput size="sm" bind:value={repo.force_branch} />
							</div>
						{/if}
					</Section>
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
					<p class="text-xs text-secondary">{displayDescription}</p>
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
