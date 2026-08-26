<script lang="ts">
	import { workspaceStore, enterpriseLicense, userStore } from '$lib/stores'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import { Loader2, Github, RotateCw, Plus, Minus, Download, AlertTriangle } from 'lucide-svelte'
	import { onDestroy } from 'svelte'
	import {
		createGitHubAppState,
		loadGithubInstallations,
		startInstallationCheck,
		stopInstallationCheck,
		addInstallationToWorkspace,
		deleteInstallation,
		exportInstallation,
		importInstallation,
		applyRepositoryURL,
		handleInstallClick,
		type GitHubAppState
	} from '$lib/githubApp'
	import RepositorySelector from './RepositorySelector.svelte'

	interface Props {
		resourceType: string
		args?: Record<string, any>
		description?: string
		onArgsUpdate?: (args: Record<string, any>) => void
		onDescriptionUpdate?: (description: string) => void
	}

	let {
		resourceType,
		args = {},
		description = '',
		onArgsUpdate,
		onDescriptionUpdate
	}: Props = $props()

	// GitHub App state using the service utilities
	let githubState: GitHubAppState = $state(createGitHubAppState())
	let githubAppPopover: { open: () => void; close: () => void } | null = $state(null)

	// Filter and deduplicate installations not in current workspace
	let githubInstallationsNotInWorkspace = $derived(
		githubState.githubInstallations
			.filter(
				(installation) =>
					!githubState.workspaceGithubInstallations.some(
						(workspaceInstallation) =>
							workspaceInstallation.installation_id === installation.installation_id
					)
			)
			.filter(
				(installation, index, array) =>
					array.findIndex((item) => item.installation_id === installation.installation_id) === index
			)
	)

	// Org names that appear on more than one installation, so the dropdown can
	// tell those entries apart.
	let duplicatedAccountIds = $derived(
		new Set(
			githubState.workspaceGithubInstallations
				.filter(
					(installation, _, array) =>
						array.filter((other) => other.account_id === installation.account_id).length > 1
				)
				.map((installation) => installation.account_id)
		)
	)

	let showGitHubApp = $derived(
		resourceType === 'git_repository' &&
			$workspaceStore &&
			($userStore?.is_admin || $userStore?.is_super_admin)
	)

	// Load GitHub installations when conditions are met
	$effect(() => {
		if (showGitHubApp && $enterpriseLicense && $workspaceStore) {
			loadGithubInstallations(githubState, $workspaceStore).catch((error) => {
				console.error('Failed to load GitHub installations:', error)
			})
		}
	})

	// Clean up interval when component is destroyed
	onDestroy(() => {
		stopInstallationCheck(githubState)
	})

	// Extracted event handlers for better maintainability
	function handleApplyRepositoryURL(close: (_: any) => void) {
		try {
			applyRepositoryURL(
				githubState,
				args,
				description,
				(newArgs) => {
					if (onArgsUpdate) {
						onArgsUpdate(newArgs)
					}
				},
				(newDescription) => {
					if (onDescriptionUpdate) {
						onDescriptionUpdate(newDescription)
					}
				}
			)
			close(null)
		} catch (error) {
			console.error('Failed to apply repository URL:', error)
		}
	}

	async function handleDeleteInstallation(installationId: number) {
		if (!$workspaceStore) return

		try {
			await deleteInstallation($workspaceStore, installationId, () =>
				loadGithubInstallations(githubState, $workspaceStore!)
			)
		} catch (error) {
			console.error('Failed to delete installation:', error)
		}
	}

	async function handleAddInstallation(installationId: number, workspaceId: string) {
		if (!$workspaceStore) return

		try {
			await addInstallationToWorkspace($workspaceStore, installationId, workspaceId, () =>
				loadGithubInstallations(githubState, $workspaceStore!)
			)
		} catch (error) {
			console.error('Failed to add installation:', error)
		}
	}

	async function handleExportInstallation(installationId: number) {
		if (!$workspaceStore) return

		try {
			await exportInstallation($workspaceStore, installationId)
		} catch (error) {
			console.error('Failed to export installation:', error)
		}
	}

	async function handleImportInstallation() {
		if (!$workspaceStore) return

		try {
			await importInstallation($workspaceStore, githubState.importJwt, () => {
				githubState.importJwt = ''
				loadGithubInstallations(githubState, $workspaceStore!)
			})
		} catch (error) {
			console.error('Failed to import installation:', error)
		}
	}

	function handleRefreshInstallations() {
		if (!$workspaceStore) return

		loadGithubInstallations(githubState, $workspaceStore).catch((error) => {
			console.error('Failed to refresh installations:', error)
		})
	}

	function handleInstallClickWithPopover() {
		if (!$workspaceStore) return

		handleInstallClick(githubState, $workspaceStore, () => {
			githubAppPopover?.open()
		})
	}
</script>

{#if showGitHubApp}
	{#if !githubState.loadingGithubInstallations}
		<Button
			variant="default"
			size="xs"
			on:click={handleRefreshInstallations}
			disabled={!$enterpriseLicense}
			startIcon={{ icon: RotateCw }}
		/>
	{:else}
		<Loader2 class="animate-spin w-10 h-4" />
	{/if}
	{#if showGitHubApp}
		<Popover
			documentationLink="https://www.windmill.dev/docs/integrations/git_repository#github-app"
			bind:this={githubAppPopover}
			disabled={!$enterpriseLicense || githubState.loadingGithubInstallations}
			contentClasses="overflow-auto"
		>
			{#snippet trigger()}
				<Button
					variant="default"
					size="xs"
					disabled={!$enterpriseLicense || githubState.loadingGithubInstallations}
					startIcon={{
						icon: githubState.loadingGithubInstallations ? Loader2 : Github,
						classes: githubState.loadingGithubInstallations ? 'animate-spin' : ''
					}}
					nonCaptureEvent
				>
					{$enterpriseLicense ? 'GitHub App' : 'GitHub App (ee only)'}
				</Button>
			{/snippet}
			{#snippet content({ close })}
				<div class="block text-primary p-4">
					<div class="flex flex-col gap-4 w-[600px]">
						{#if githubState.workspaceGithubInstallations.length > 0}
							<div class="flex flex-col gap-2">
								<p class="text-sm font-semibold text-secondary">Select Repository</p>
								<div class="flex flex-row gap-2 w-full">
									<div class="flex flex-col gap-1 flex-1">
										<p class="text-sm font-semibold text-secondary">GitHub Account ID</p>
										<select
											bind:value={githubState.selectedGHAppInstallationId}
											onchange={() => (githubState.selectedGHAppRepository = undefined)}
										>
											<option value={undefined} disabled>Select GitHub Account ID</option>
											{#each githubState.workspaceGithubInstallations as installation (`select-${installation.installation_id}-${installation.workspace_id}`)}
												{@const details = [
													duplicatedAccountIds.has(installation.account_id)
														? `${installation.installation_id}`
														: undefined,
													installation.error ? 'token error' : undefined
												].filter(Boolean)}
												<option
													value={installation.installation_id}
													disabled={!!installation.error}
												>
													{installation.account_id}{details.length
														? ` (${details.join(', ')})`
														: ''}
												</option>
											{/each}
										</select>
									</div>
									{#if githubState.selectedGHAppInstallationId !== undefined}
										{@const selectedInstallation = githubState.workspaceGithubInstallations.find(
											(inst) => inst.installation_id === githubState.selectedGHAppInstallationId
										)}
										{#if selectedInstallation}
											<div class="flex flex-col gap-1 flex-1">
												<p class="text-sm font-semibold text-secondary">Repository</p>
												<!-- RepositorySelector snapshots its repositories and page cursor at
												     mount, so switching installation has to remount it. -->
												{#key selectedInstallation.installation_id}
													<RepositorySelector
														bind:selectedRepository={githubState.selectedGHAppRepository}
														installationId={selectedInstallation.installation_id}
														initialRepositories={selectedInstallation.repositories}
														totalCount={selectedInstallation.total_count}
														perPage={selectedInstallation.per_page}
													/>
												{/key}
											</div>
										{/if}
									{/if}
									<div class="pt-[26px]">
										<Button
											size="xs"
											variant="accent"
											buttonType="button"
											disabled={!githubState.selectedGHAppRepository}
											on:click={() => handleApplyRepositoryURL(close)}
										>
											Apply
										</Button>
									</div>
								</div>
							</div>
						{/if}

						<div
							class={`${
								githubState.workspaceGithubInstallations.length > 0
									? 'border-t border-gray-200 dark:border-gray-700'
									: ''
							} pt-4`}
						>
							<div class="flex flex-col gap-4">
								<div class="flex">
									<Button
										variant="default"
										size="xs"
										href={githubState.githubInstallationUrl}
										startIcon={{
											icon: githubState.isCheckingInstallation ? Loader2 : Plus,
											classes: githubState.isCheckingInstallation ? 'animate-spin' : ''
										}}
										target="_blank"
										disabled={githubState.isCheckingInstallation}
										on:click={() => {
											if ($workspaceStore) {
												startInstallationCheck(githubState, $workspaceStore, () =>
													loadGithubInstallations(githubState, $workspaceStore!)
												)
											}
										}}
									>
										{githubState.isCheckingInstallation
											? 'Checking for new installations...'
											: 'Add new installation'}
									</Button>
								</div>
								{#if githubState.workspaceGithubInstallations.length > 0}
									<div class="flex flex-col gap-2">
										<p class="text-sm font-semibold text-secondary">Current installations:</p>
										<div class="flex flex-col gap-1">
											<table class="w-full text-sm">
												<thead>
													<tr class="text-left text-xs text-primary">
														<th class="pb-2 w-1/3">Org</th>
														<th class="pb-2 w-1/6">Workspace</th>
														<th class="pb-2 w-1/6">Repos</th>
														<th class="pb-2 w-1/3"></th>
													</tr>
												</thead>
												<tbody>
													{#each githubState.workspaceGithubInstallations as installation (`current-${installation.installation_id}-${installation.workspace_id}`)}
														<tr class="border-t border-gray-200 dark:border-gray-700">
															<td class="py-2">
																<div class="flex items-center gap-1 flex-wrap">
																	{#if installation.error}
																		<span title={installation.error}>
																			<AlertTriangle class="w-4 h-4 text-yellow-500" />
																		</span>
																	{/if}
																	{installation.account_id}
																	{#if installation.provisioned_by_admin}
																		<span
																			class="text-2xs px-1.5 py-0.5 rounded bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200"
																			title="Assigned by the instance super-admin from instance settings. Only the super-admin can remove it."
																		>
																			Provisioned by admin
																		</span>
																	{/if}
																</div>
															</td>
															<td class="py-2">
																<span class="text-xs text-primary">{installation.workspace_id}</span
																>
															</td>
															<td class="py-2 text-primary">
																{#if installation.error}
																	<span
																		class="text-yellow-600 dark:text-yellow-400 text-xs"
																		title={installation.error}>Token error</span
																	>
																{:else}
																	{installation.repositories.length} repos
																{/if}
															</td>
															<td class="py-2 text-right">
																<div class="flex justify-end gap-1">
																	{#if !installation.github_base_url}
																		<Button
																			size="xs2"
																			variant="accent"
																			title="Export installation to other instance"
																			startIcon={{ icon: Download }}
																			on:click={() =>
																				handleExportInstallation(installation.installation_id)}
																		>
																			Export
																		</Button>
																	{/if}
																	{#if !installation.provisioned_by_admin}
																		<Button
																			size="xs2"
																			variant="default"
																			destructive
																			title="Remove installation from workspace"
																			startIcon={{ icon: Minus }}
																			on:click={() =>
																				handleDeleteInstallation(installation.installation_id)}
																		>
																			Remove
																		</Button>
																	{/if}
																</div>
															</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</div>
									</div>
								{/if}
								{#if githubInstallationsNotInWorkspace.length > 0}
									<div class="flex flex-col gap-2">
										<p class="text-sm font-semibold text-secondary"
											>Installations in other workspaces:</p
										>
										<div class="flex flex-col gap-1">
											<table class="w-full text-sm">
												<thead>
													<tr class="text-left text-xs text-primary">
														<th class="pb-2 w-1/3">Org</th>
														<th class="pb-2 w-1/6">Workspace</th>
														<th class="pb-2 w-1/6">Repos</th>
														<th class="pb-2 w-1/3"></th>
													</tr>
												</thead>
												<tbody>
													{#each githubInstallationsNotInWorkspace as installation (`other-${installation.installation_id}-${installation.workspace_id}`)}
														<tr class="border-t border-gray-200 dark:border-gray-700">
															<td class="py-2">
																<div class="flex items-center gap-1">
																	{#if installation.error}
																		<span title={installation.error}>
																			<AlertTriangle class="w-4 h-4 text-yellow-500" />
																		</span>
																	{/if}
																	{installation.account_id}
																</div>
															</td>
															<td class="py-2">
																<span class="text-xs text-primary">{installation.workspace_id}</span
																>
															</td>
															<td class="py-2 text-primary">
																{#if installation.error}
																	<span
																		class="text-yellow-600 dark:text-yellow-400 text-xs"
																		title={installation.error}>Token error</span
																	>
																{:else}
																	{installation.repositories.length} repos
																{/if}
															</td>
															<td class="pl-8 py-2 text-right">
																<Button
																	size="xs2"
																	variant="accent"
																	title="Add installation to workspace"
																	startIcon={{ icon: Plus }}
																	on:click={() => {
																		if (installation.workspace_id) {
																			handleAddInstallation(
																				installation.installation_id,
																				installation.workspace_id
																			)
																		}
																	}}
																>
																	Add to workspace
																</Button>
															</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</div>
									</div>
								{/if}
							</div>
						</div>

						{#if !githubState.isGhesSelfManaged}
							<div class="mt-4 flex flex-col gap-2">
								<p class="text-sm font-semibold text-secondary"
									>Import installation from other instance:</p
								>
								<div class="flex gap-2">
									<input
										type="text"
										placeholder="Paste JWT token here"
										bind:value={githubState.importJwt}
										class="flex-1"
									/>
									<Button
										variant="accent"
										on:click={handleImportInstallation}
										disabled={!githubState.importJwt}
									>
										Import
									</Button>
								</div>
							</div>
						{/if}
					</div>
				</div>
			{/snippet}
		</Popover>
	{:else}
		<Button
			variant="default"
			size="xs"
			disabled={!$enterpriseLicense || githubState.loadingGithubInstallations}
			startIcon={{
				icon:
					githubState.loadingGithubInstallations || githubState.isCheckingInstallation
						? Loader2
						: Github,
				classes:
					githubState.loadingGithubInstallations || githubState.isCheckingInstallation
						? 'animate-spin'
						: ''
			}}
			href={githubState.githubInstallationUrl}
			target="_blank"
			on:click={handleInstallClickWithPopover}
		>
			{$enterpriseLicense
				? githubState.isCheckingInstallation
					? 'Waiting for installation...'
					: 'Install GitHub App'
				: 'GitHub App (ee only)'}
		</Button>
	{/if}
{/if}
