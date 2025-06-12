<script lang="ts">
	import {
		OauthService,
		GitSyncService,
		type ResourceType,
		type GetGlobalConnectedRepositoriesResponse
	} from '$lib/gen'
	import {
		workspaceStore,
		enterpriseLicense,
		userWorkspaces,
		userStore,
		workspaceColor
	} from '$lib/stores'
	import { base } from '$lib/base'
	import { emptySchema, emptyString } from '$lib/utils'
	import SchemaForm from './SchemaForm.svelte'
	import type SimpleEditor from './SimpleEditor.svelte'
	import Toggle from './Toggle.svelte'
	import TestConnection from './TestConnection.svelte'
	import SupabaseIcon from './icons/SupabaseIcon.svelte'
	import { sendUserToast } from '$lib/toast'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import { Loader2, Github, RotateCw, Plus, Minus, Download } from 'lucide-svelte'
	import { onDestroy, untrack } from 'svelte'

	interface Props {
		resourceType: string
		resourceTypeInfo: ResourceType | undefined
		args?: Record<string, any> | any
		linkedSecret?: string | undefined
		isValid?: boolean
		linkedSecretCandidates?: string[] | undefined
		description?: string | undefined
	}

	let {
		resourceType,
		resourceTypeInfo,
		args = $bindable({}),
		linkedSecret = $bindable(undefined),
		isValid = $bindable(true),
		linkedSecretCandidates = undefined,
		description = $bindable(undefined)
	}: Props = $props()

	let schema = $state(emptySchema())
	let notFound = $state(false)

	let supabaseWizard = $state(false)

	let loadingGithubInstallations = $state(false)
	let githubInstallations: GetGlobalConnectedRepositoriesResponse = $state([])
	let workspaceGithubInstallations: GetGlobalConnectedRepositoriesResponse = $state([])
	let selectedGHAppAccountId: string | undefined = $state(undefined)
	let selectedGHAppRepository: string | undefined = $state(undefined)
	let githubInstallationUrl: string | undefined = $state(undefined)
	let installationCheckInterval: number | undefined = undefined
	let isCheckingInstallation = $state(false)
	let importJwt = $state('')
	let githubAppPopover: { open: () => void; close: () => void } | null = $state(null)

	async function loadGithubInstallations() {
		if (!$enterpriseLicense) return
		try {
			loadingGithubInstallations = true

			// Reset for reactivity
			githubInstallations = []
			workspaceGithubInstallations = []

			const installations = await GitSyncService.getGlobalConnectedRepositories()
			githubInstallations = installations
			workspaceGithubInstallations = githubInstallations.filter(
				(_) => _.workspace_id === $workspaceStore
			)

			const state = encodeURIComponent(
				JSON.stringify({
					workspace_id: $workspaceStore,
					base_url: window.location.origin + base
				})
			)

			githubInstallationUrl = `https://github.com/apps/windmill-sync-helper/installations/new?state=${state}`
		} catch (err) {
			console.error(err)
			sendUserToast('Failed to load GitHub installations', true)
			githubInstallations = []
			workspaceGithubInstallations = []
		} finally {
			loadingGithubInstallations = false
		}
	}

	function startInstallationCheck() {
		isCheckingInstallation = true
		installationCheckInterval = window.setInterval(async () => {
			const installations = await GitSyncService.getGlobalConnectedRepositories()
			if (installations.length > 0) {
				stopInstallationCheck()
				githubInstallations = installations
				workspaceGithubInstallations = githubInstallations.filter(
					(_) => _.workspace_id === $workspaceStore
				)
				// Open the popover with a small delay as otherwise it doesn't open
				setTimeout(() => {
					githubAppPopover?.open()
				}, 100)
			}
		}, 2000)
	}

	function stopInstallationCheck() {
		if (installationCheckInterval) {
			clearInterval(installationCheckInterval)
			installationCheckInterval = undefined
		}
		isCheckingInstallation = false
	}

	// Clean up interval when component is destroyed
	onDestroy(() => {
		stopInstallationCheck()
	})

	function getRepositories(accountId: string) {
		return githubInstallations.find((_) => _.account_id === accountId)?.repositories || []
	}

	async function addInstallationToWorkspace(
		installation_id: number | undefined,
		workspaceId: string | undefined
	) {
		if (!installation_id || !workspaceId || !$workspaceStore) {
			sendUserToast('Installation or workspace invalid', true)
			return
		}
		try {
			await GitSyncService.installFromWorkspace({
				workspace: $workspaceStore,
				requestBody: {
					source_workspace_id: workspaceId,
					installation_id: installation_id
				}
			})
			sendUserToast('Successfully added installation to workspace', false)
			await loadGithubInstallations()
		} catch (err) {
			console.error(err)
			sendUserToast('Failed to add installation to workspace', true)
		}
	}

	async function isSupabaseAvailable() {
		try {
			supabaseWizard =
				((await OauthService.listOauthConnects()) ?? {})['supabase_wizard'] != undefined
		} catch (error) {}
	}
	async function loadSchema() {
		if (!resourceTypeInfo) return
		rawCode = '{}'
		viewJsonSchema = false
		try {
			schema = resourceTypeInfo.schema as any
			schema.order = schema.order ?? Object.keys(schema.properties).sort()
			notFound = false
		} catch (e) {
			notFound = true
		}
	}

	function parseJson() {
		try {
			args = JSON.parse(rawCode)
			error = ''
			isValid = true
		} catch (e) {
			isValid = false
			error = e.message
		}
	}
	let error = $state('')
	let rawCode = $state('')
	let viewJsonSchema = $state(false)

	function switchTab(asJson: boolean) {
		viewJsonSchema = asJson
		if (asJson) {
			rawCode = JSON.stringify(args, null, 2)
		} else {
			parseJson()
			if (resourceTypeInfo?.format_extension) {
				textFileContent = args.content
			}
		}
	}

	let connectionString = $state('')
	let validConnectionString = $state(true)
	function parseConnectionString(close: (_: any) => void) {
		const regex =
			/postgres(?:ql)?:\/\/(?<user>[^:@]+)(?::(?<password>[^@]+))?@(?<host>[^:\/?]+)(?::(?<port>\d+))?\/(?<dbname>[^\?]+)?(?:\?.*sslmode=(?<sslmode>[^&]+))?/
		const match = connectionString.match(regex)
		if (match) {
			validConnectionString = true
			const { user, password, host, port, dbname, sslmode } = match.groups!
			rawCode = JSON.stringify(
				{
					...args,
					user,
					password: password || args?.password,
					host,
					port: (port ? Number(port) : undefined) || args?.port,
					dbname: dbname || args?.dbname,
					sslmode: sslmode || args?.sslmode
				},
				null,
				2
			)
			rawCodeEditor?.setCode(rawCode)
			close(null)
		} else {
			validConnectionString = false
		}
	}

	function applyRepositoryURL(close: (_: any) => void) {
		if (!selectedGHAppRepository) return
		rawCode = JSON.stringify(
			{
				...args,
				url: selectedGHAppRepository,
				is_github_app: true
			},
			null,
			2
		)
		description = `Repository ${selectedGHAppRepository} with permissions fetched using Windmill Github App. ${description ?? ''}`
		rawCodeEditor?.setCode(rawCode)
		close(null)
	}

	let rawCodeEditor: SimpleEditor | undefined = $state(undefined)
	let textFileContent: string | undefined = $state(undefined)

	function parseTextFileContent() {
		args = {
			content: textFileContent
		}
	}

	async function deleteInstallation(installation_id: number) {
		if (!$workspaceStore) {
			sendUserToast('Failed to delete installation', true)
			return
		}
		try {
			await GitSyncService.deleteFromWorkspace({
				workspace: $workspaceStore,
				installationId: installation_id
			})
			sendUserToast('Successfully deleted installation', false)
			await loadGithubInstallations()
		} catch (err) {
			console.error(err)
			sendUserToast('Failed to delete installation', true)
		}
	}

	async function exportInstallation(installationId: number) {
		if (!$workspaceStore) {
			sendUserToast('Failed to export installation', true)
			return
		}
		try {
			const response = await GitSyncService.exportInstallation({
				workspace: $workspaceStore,
				installationId: installationId
			})
			if (!response.jwt_token) {
				sendUserToast('Failed to export installation', true)
				return
			}
			// Copy to clipboard
			await navigator.clipboard.writeText(response.jwt_token)
			sendUserToast(
				'JWT token copied to clipboard. This token is sensitive and should be kept secret!',
				false,
				undefined,
				undefined,
				10000
			)
		} catch (error) {
			console.error(error)
			sendUserToast('Failed to export installation', true)
		}
	}

	async function importInstallation(jwt: string) {
		if (!$workspaceStore) {
			sendUserToast('Failed to import installation', true)
			return
		}
		try {
			await GitSyncService.importInstallation({
				workspace: $workspaceStore,
				requestBody: { jwt_token: jwt }
			})
			importJwt = ''
			sendUserToast('Installation imported successfully', false)
			await loadGithubInstallations()
		} catch (error) {
			sendUserToast('Failed to import installation', true)
		}
	}

	function handleInstallClick() {
		if (githubInstallations.length === 0) {
			if (!isCheckingInstallation) {
				startInstallationCheck()
			}
		}
	}
	$effect(() => {
		$workspaceStore && untrack(() => loadSchema())
	})
	$effect(() => {
		notFound && rawCode && untrack(() => parseJson())
	})
	$effect(() => {
		rawCode && untrack(() => parseJson())
	})
	$effect(() => {
		textFileContent && untrack(() => parseTextFileContent())
	})
	$effect(() => {
		resourceType == 'postgresql' && untrack(() => isSupabaseAvailable())
	})
	$effect(() => {
		resourceType == 'git_repository' &&
			$userStore?.is_admin &&
			untrack(() => loadGithubInstallations())
	})
	let githubInstallationsNotInWorkspace = $derived(
		githubInstallations.filter((installation) => {
			return !workspaceGithubInstallations.some(
				(workspaceInstallation) =>
					workspaceInstallation.installation_id === installation.installation_id
			)
		})
	)
</script>

{#if !notFound}
	<div class="w-full flex gap-4 flex-row-reverse items-center">
		<Toggle
			on:change={(e) => switchTab(e.detail)}
			options={{
				right: 'As JSON'
			}}
		/>
		<TestConnection {resourceType} {args} />
		{#if resourceType == 'postgresql'}
			<Popover
				floatingConfig={{
					placement: 'bottom'
				}}
			>
				{#snippet trigger()}
					<Button
						spacingSize="sm"
						size="xs"
						btnClasses="h-8"
						color="light"
						variant="border"
						nonCaptureEvent
					>
						From connection string
					</Button>
				{/snippet}
				{#snippet content({ close })}
					<div class="block text-primary p-4">
						<div class="w-[550px] flex flex-col items-start gap-1">
							<div class="flex flex-row gap-1 w-full">
								<input
									type="text"
									bind:value={connectionString}
									placeholder="postgres://user:password@host:5432/dbname?sslmode=disable"
								/>
								<Button
									size="xs"
									color="blue"
									buttonType="button"
									on:click={() => {
										parseConnectionString(close)
									}}
									disabled={connectionString.length <= 0}
								>
									Apply
								</Button>
							</div>
							{#if !validConnectionString}
								<p class="text-red-500 text-xs">Could not parse connection string</p>
							{/if}
						</div>
					</div>
				{/snippet}
			</Popover>
		{/if}
		{#if resourceType == 'postgresql' && supabaseWizard}
			<a
				target="_blank"
				href="{base}/api/oauth/connect/supabase_wizard"
				class="border rounded-lg flex flex-row gap-2 items-center text-xs px-3 py-1.5 h-8 bg-[#F1F3F5] hover:bg-[#E6E8EB] dark:bg-[#1C1C1C] dark:hover:bg-black"
			>
				<SupabaseIcon height="16px" width="16px" />
				<div class="text-[#11181C] dark:text-[#EDEDED] font-semibold">Connect Supabase</div>
			</a>
		{/if}
		{#if resourceType == 'git_repository' && $workspaceStore && $userStore?.is_admin}
			{#if !loadingGithubInstallations}
				<Button
					color="light"
					variant="contained"
					size="xs"
					on:click={loadGithubInstallations}
					disabled={!$enterpriseLicense}
					startIcon={{ icon: RotateCw }}
				/>
			{:else}
				<Loader2 class="animate-spin w-10 h-4" />
			{/if}
			{#if githubInstallations.length > 0}
				<Popover
					documentationLink="https://www.windmill.dev/docs/integrations/git_repository#github-app"
					bind:this={githubAppPopover}
					floatingConfig={{
						placement: 'bottom'
					}}
					disabled={!$enterpriseLicense || loadingGithubInstallations}
				>
					{#snippet trigger()}
						<Button
							color="none"
							variant="border"
							size="xs"
							disabled={!$enterpriseLicense || loadingGithubInstallations}
							startIcon={{
								icon: loadingGithubInstallations ? Loader2 : Github,
								classes: loadingGithubInstallations ? 'animate-spin' : ''
							}}
							nonCaptureEvent
						>
							{$enterpriseLicense ? 'GitHub App' : 'GitHub App (ee only)'}
						</Button>
					{/snippet}
					{#snippet content({ close })}
						<div class="block text-primary p-4">
							<div class="flex flex-col gap-4 w-[600px]">
								{#if workspaceGithubInstallations.length > 0}
									<div class="flex flex-col gap-2">
										<p class="text-sm font-semibold text-secondary">Select Repository</p>
										<div class="flex flex-row gap-2 w-full">
											<div class="flex flex-col gap-1 flex-1">
												<p class="text-sm font-semibold text-secondary">Github Account ID</p>
												<select bind:value={selectedGHAppAccountId}>
													<option value="" disabled>Select GitHub Account ID</option>
													{#each workspaceGithubInstallations as installation}
														<option value={installation.account_id}
															>{installation.account_id}</option
														>
													{/each}
												</select>
											</div>
											{#if selectedGHAppAccountId}
												<div class="flex flex-col gap-1 flex-1">
													<p class="text-sm font-semibold text-secondary">Repository</p>
													<div class="flex flex-row gap-2">
														<select bind:value={selectedGHAppRepository}>
															<option value="" disabled selected>Select Repository</option>
															{#each getRepositories(selectedGHAppAccountId) as repository}
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
													disabled={!selectedGHAppRepository}
													on:click={() => {
														applyRepositoryURL(close)
													}}
												>
													Apply
												</Button>
											</div>
										</div>
									</div>
								{/if}

								<div
									class={`${
										workspaceGithubInstallations.length > 0
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
												href={githubInstallationUrl}
												startIcon={{ icon: Plus }}
												target="_blank"
											>
												Add new installation
											</Button>
										</div>
										{#if workspaceGithubInstallations.length > 0}
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
															{#each workspaceGithubInstallations as installation}
																<tr class="border-t border-gray-200 dark:border-gray-700">
																	<td class="py-2">{installation.account_id}</td>
																	<td class="py-2">
																		{#if $workspaceColor}
																			<span
																				class="inline-flex items-center px-2 py-0.5 rounded text-xs"
																				style="background-color: {$workspaceColor}20; color: {$workspaceColor}"
																			>
																				{installation.workspace_id}
																			</span>
																		{:else}
																			<span class="text-xs text-tertiary"
																				>{installation.workspace_id}</span
																			>
																		{/if}
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
																					exportInstallation(installation.installation_id)}
																			>
																				Export
																			</Button>
																			<Button
																				size="xs2"
																				color="red"
																				title="Remove installation from workspace"
																				startIcon={{ icon: Minus }}
																				on:click={() =>
																					deleteInstallation(installation.installation_id)}
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
															{#each githubInstallationsNotInWorkspace as installation}
																<tr class="border-t border-gray-200 dark:border-gray-700">
																	<td class="py-2">{installation.account_id}</td>
																	<td class="py-2">
																		{#if $userWorkspaces.find((w) => w.id === installation.workspace_id)?.color}
																			<span
																				class="inline-flex items-center px-2 py-0.5 rounded text-xs"
																				style="background-color: {$userWorkspaces.find(
																					(w) => w.id === installation.workspace_id
																				)?.color}20; color: {$userWorkspaces.find(
																					(w) => w.id === installation.workspace_id
																				)?.color}"
																			>
																				{installation.workspace_id}
																			</span>
																		{:else}
																			<span class="text-xs text-tertiary"
																				>{installation.workspace_id}</span
																			>
																		{/if}
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
																				addInstallationToWorkspace(
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
											bind:value={importJwt}
											class="flex-1"
										/>
										<Button
											color="blue"
											on:click={() => importInstallation(importJwt)}
											disabled={!importJwt}
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
					disabled={!$enterpriseLicense || loadingGithubInstallations}
					startIcon={{
						icon: loadingGithubInstallations || isCheckingInstallation ? Loader2 : Github,
						classes: loadingGithubInstallations || isCheckingInstallation ? 'animate-spin' : ''
					}}
					href={githubInstallationUrl}
					target="_blank"
					on:click={handleInstallClick}
				>
					{$enterpriseLicense
						? isCheckingInstallation
							? 'Waiting for installation...'
							: 'Install GitHub App'
						: 'GitHub App (ee only)'}
				</Button>
			{/if}
		{/if}
	</div>
{:else}
	<p class="italic text-tertiary text-xs mb-4"
		>No corresponding resource type found in your workspace for {resourceType}. Define the value in
		JSON directly</p
	>
{/if}
{#if notFound || viewJsonSchema}
	{#if !emptyString(error)}<span class="text-red-400 text-xs mb-1 flex flex-row-reverse"
			>{error}</span
		>{:else}<div class="py-2"></div>{/if}
	<div class="h-full w-full border p-1 rounded">
		{#await import('$lib/components/SimpleEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				bind:this={rawCodeEditor}
				autoHeight
				lang="json"
				bind:code={rawCode}
				fixedOverflowWidgets={false}
			/>
		{/await}
	</div>
{:else if resourceTypeInfo?.format_extension}
	<h5 class="mt-4 inline-flex items-center gap-4">
		File content ({resourceTypeInfo.format_extension})
	</h5>
	<div class="py-2"></div>
	<div class="h-full w-full border p-1 rounded">
		{#await import('$lib/components/SimpleEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				bind:this={rawCodeEditor}
				autoHeight
				lang={resourceTypeInfo.format_extension}
				bind:code={textFileContent}
				fixedOverflowWidgets={false}
			/>
		{/await}
	</div>
{:else}
	<SchemaForm
		onlyMaskPassword
		noDelete
		{linkedSecretCandidates}
		bind:linkedSecret
		isValid
		{schema}
		bind:args
	/>
{/if}
