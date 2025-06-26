<script lang="ts">
	import { workspaceStore, enterpriseLicense, userStore } from '$lib/stores'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import { Loader2, Github, RotateCw, Plus, Minus, Download } from 'lucide-svelte'
	import { onDestroy } from 'svelte'
	import {
		createGitHubAppState,
		loadGithubInstallations,
		startInstallationCheck,
		stopInstallationCheck,
		getRepositories,
		addInstallationToWorkspace,
		deleteInstallation,
		exportInstallation,
		importInstallation,
		applyRepositoryURL,
		handleInstallClick,
		type GitHubAppState
	} from '$lib/githubApp.ts'

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

	let showGitHubApp = $derived(
		resourceType === 'git_repository' && $workspaceStore && $userStore?.is_admin
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
			color="light"
			variant="contained"
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
			floatingConfig={{
				placement: 'bottom'
			}}
			disabled={!$enterpriseLicense || githubState.loadingGithubInstallations}
		>
			{#snippet trigger()}
				<Button
					color="none"
					variant="border"
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
										<p class="text-sm font-semibold text-secondary">Github Account ID</p>
										<select bind:value={githubState.selectedGHAppAccountId}>
											<option value="" disabled>Select GitHub Account ID</option>
											{#each githubState.workspaceGithubInstallations as installation (`select-${installation.installation_id}-${installation.workspace_id}`)}
												<option value={installation.account_id}>{installation.account_id}</option>
											{/each}
										</select>
									</div>
									{#if githubState.selectedGHAppAccountId}
										<div class="flex flex-col gap-1 flex-1">
											<p class="text-sm font-semibold text-secondary">Repository</p>
											<div class="flex flex-row gap-2">
												<select bind:value={githubState.selectedGHAppRepository}>
													<option value="" disabled selected>Select Repository</option>
													{#each getRepositories(githubState, githubState.selectedGHAppAccountId) as repository (repository.url)}
														<option value={repository.url}>{repository.name}</option>
													{/each}
												</select>
											</div>
										</div>
									{/if}
									<div class="pt-[26px]">
										<Button
											size="xs"
											color="blue"
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
										color="none"
										variant="border"
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
													<tr class="text-left text-xs text-tertiary">
														<th class="pb-2 w-1/3">Org</th>
														<th class="pb-2 w-1/6">Workspace</th>
														<th class="pb-2 w-1/6">Repos</th>
														<th class="pb-2 w-1/3"></th>
													</tr>
												</thead>
												<tbody>
													{#each githubState.workspaceGithubInstallations as installation (`current-${installation.installation_id}-${installation.workspace_id}`)}
														<tr class="border-t border-gray-200 dark:border-gray-700">
															<td class="py-2">{installation.account_id}</td>
															<td class="py-2">
																<span class="text-xs text-tertiary"
																	>{installation.workspace_id}</span
																>
															</td>
															<td class="py-2 text-tertiary">
																{installation.repositories.length} repos
															</td>
															<td class="py-2 text-right">
																<div class="flex justify-end gap-1">
																	<Button
																		size="xs2"
																		color="blue"
																		title="Export installation to other instance"
																		startIcon={{ icon: Download }}
																		on:click={() =>
																			handleExportInstallation(installation.installation_id)}
																	>
																		Export
																	</Button>
																	<Button
																		size="xs2"
																		color="red"
																		title="Remove installation from workspace"
																		startIcon={{ icon: Minus }}
																		on:click={() =>
																			handleDeleteInstallation(installation.installation_id)}
																	>
																		Remove
																	</Button>
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
													<tr class="text-left text-xs text-tertiary">
														<th class="pb-2 w-1/3">Org</th>
														<th class="pb-2 w-1/6">Workspace</th>
														<th class="pb-2 w-1/6">Repos</th>
														<th class="pb-2 w-1/3"></th>
													</tr>
												</thead>
												<tbody>
													{#each githubInstallationsNotInWorkspace as installation (`other-${installation.installation_id}-${installation.workspace_id}`)}
														<tr class="border-t border-gray-200 dark:border-gray-700">
															<td class="py-2">{installation.account_id}</td>
															<td class="py-2">
																<span class="text-xs text-tertiary"
																	>{installation.workspace_id}</span
																>
															</td>
															<td class="py-2 text-tertiary">
																{installation.repositories.length} repos
															</td>
															<td class="pl-8 py-2 text-right">
																<Button
																	size="xs2"
																	color="blue"
																	title="Add installation to workspace"
																	startIcon={{ icon: Plus }}
																	on:click={() =>
																		handleAddInstallation(
																			installation.installation_id,
																			installation.workspace_id
																		)}
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
									color="blue"
									on:click={handleImportInstallation}
									disabled={!githubState.importJwt}
								>
									Import
								</Button>
							</div>
						</div>
					</div>
				</div>
			{/snippet}
		</Popover>
	{:else}
		<Button
			color="none"
			variant="border"
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
