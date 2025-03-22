<script lang="ts">
	import {
		OauthService,
		GitSyncService,
		type ResourceType,
		type GetGlobalConnectedRepositoriesResponse
	} from '$lib/gen'
	import { workspaceStore, enterpriseLicense, userWorkspaces } from '$lib/stores'
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
	import { Loader2, Github, RotateCw, Plus, Minus } from 'lucide-svelte'

	export let resourceType: string
	export let resourceTypeInfo: ResourceType | undefined
	export let args: Record<string, any> | any = {}
	export let linkedSecret: string | undefined = undefined
	export let isValid = true
	export let linkedSecretCandidates: string[] | undefined = undefined

	let schema = emptySchema()
	let notFound = false

	let supabaseWizard = false

	let loadingGithubInstallations = false
	let githubInstallations: GetGlobalConnectedRepositoriesResponse = []
	let workspaceGithubInstallations: GetGlobalConnectedRepositoriesResponse = []
	let selectedGHAppAccountId: string | undefined = undefined
	let selectedGHAppRepository: string | undefined = undefined

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
		} catch (err) {
			console.error(err)
			sendUserToast('Failed to load GitHub installations', true)
			githubInstallations = []
			workspaceGithubInstallations = []
		} finally {
			loadingGithubInstallations = false
		}
	}

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
	$: $workspaceStore && loadSchema()

	$: notFound && rawCode && parseJson()

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
	let error = ''
	let rawCode = ''
	let viewJsonSchema = false

	$: rawCode && parseJson()

	$: textFileContent && parseTextFileContent()

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

	$: resourceType == 'postgresql' && isSupabaseAvailable()
	$: resourceType == 'git_repository' && loadGithubInstallations()

	let connectionString = ''
	let validConnectionString = true
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
				github_app_installation_id: githubInstallations.find(
					(_) => _.account_id === selectedGHAppAccountId
				)?.installation_id,
				is_github_app: true
			},
			null,
			2
		)
		rawCodeEditor?.setCode(rawCode)
		close(null)
	}

	let rawCodeEditor: SimpleEditor | undefined = undefined
	let textFileContent: string

	function parseTextFileContent() {
		args = {
			content: textFileContent
		}
	}

	$: githubInstallationsNotInWorkspace = githubInstallations.filter((installation) => {
		return !workspaceGithubInstallations.some(
			(workspaceInstallation) =>
				workspaceInstallation.installation_id === installation.installation_id
		)
	})

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
				<svelte:fragment slot="trigger">
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
				</svelte:fragment>
				<svelte:fragment slot="content" let:close>
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
				</svelte:fragment>
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
		{#if resourceType == 'git_repository' && $workspaceStore}
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
			<Popover
				floatingConfig={{
					placement: 'bottom'
				}}
				disabled={workspaceGithubInstallations.length == 0}
			>
				<svelte:fragment slot="trigger">
					<Button
						spacingSize="sm"
						size="xs"
						btnClasses="h-8"
						color="light"
						variant="border"
						nonCaptureEvent
					>
						From connected repository
					</Button>
				</svelte:fragment>
				<svelte:fragment slot="content" let:close>
					<div class="block text-primary p-4">
						<div class="flex flex-row gap-2 w-full">
							<div class="flex flex-col gap-1 flex-1">
								<p class="text-sm font-semibold text-secondary">Github Account ID</p>
								<select bind:value={selectedGHAppAccountId}>
									<option value="" disabled>Select GitHub Account ID</option>
									{#each workspaceGithubInstallations as installation}
										<option value={installation.account_id}>{installation.account_id}</option>
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
				</svelte:fragment>
			</Popover>

			{#if githubInstallationsNotInWorkspace.length > 0 || workspaceGithubInstallations.length > 0}
				<Popover
					floatingConfig={{
						placement: 'bottom'
					}}
					disabled={!$enterpriseLicense || loadingGithubInstallations}
				>
					<svelte:fragment slot="trigger">
						<Button
							color="none"
							variant="border"
							size="xs"
							disabled={!$enterpriseLicense || loadingGithubInstallations}
							startIcon={{ icon: loadingGithubInstallations ? Loader2 : Github }}
							nonCaptureEvent
						>
							{$enterpriseLicense ? 'Manage GitHub app installations' : 'GitHub app (ee only)'}
						</Button>
					</svelte:fragment>
					<svelte:fragment slot="content">
						<div class="block text-primary p-4">
							<div class="flex flex-col gap-4 w-[300px]">
								<Button
									color="none"
									variant="border"
									size="xs"
									on:click={async () => {
										if ($workspaceStore) {
											const state = encodeURIComponent(
												JSON.stringify({
													workspace_id: $workspaceStore,
													base_url: window.location.origin + base
												})
											)
											window.open(
												`https://github.com/apps/windmill-sync-helper/installations/new?state=${state}`,
												'_blank'
											)
										} else {
											sendUserToast('Failed to get GitHub app installation URL', true)
										}
									}}
								>
									Add new installation
								</Button>
								{#if workspaceGithubInstallations.length > 0}
									<div class="flex flex-col gap-2">
										<p class="text-sm font-semibold text-secondary">Current installations:</p>
										<div class="flex flex-col gap-1">
											<table class="w-full text-sm">
												<thead>
													<tr class="text-left text-xs text-tertiary">
														<th class="pb-2 w-1/3">Org</th>
														<th class="pb-2 w-1/3">Workspace</th>
														<th class="pb-2 w-1/4">Repos</th>
														<th class="pb-2 w-1/12" />
													</tr>
												</thead>
												<tbody>
													{#each workspaceGithubInstallations as installation}
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
															<td class="pl-2 py-2 text-right">
																<Button
																	size="xs2"
																	color="red"
																	iconOnly
																	btnClasses="px-1"
																	startIcon={{ icon: Minus }}
																	on:click={() => deleteInstallation(installation.installation_id)}
																/>
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
														<th class="pb-2 w-1/3">Workspace</th>
														<th class="pb-2 w-1/4">Repos</th>
														<th class="pb-2 w-1/12" />
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
															<td class="pl-2 py-2 text-right">
																<Button
																	size="xs2"
																	color="blue"
																	iconOnly
																	btnClasses="px-1"
																	startIcon={{ icon: Plus }}
																	on:click={() =>
																		addInstallationToWorkspace(
																			installation.installation_id,
																			installation.workspace_id
																		)}
																/>
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
					</svelte:fragment>
				</Popover>
			{:else}
				<Button
					color="none"
					variant="border"
					size="xs"
					disabled={!$enterpriseLicense || loadingGithubInstallations}
					startIcon={{ icon: loadingGithubInstallations ? Loader2 : Github }}
					on:click={async () => {
						if ($workspaceStore) {
							const state = encodeURIComponent(
								JSON.stringify({
									workspace_id: $workspaceStore,
									base_url: window.location.origin + base
								})
							)
							window.open(
								`https://github.com/apps/windmill-sync-helper/installations/new?state=${state}`,
								'_blank'
							)
						} else {
							sendUserToast('Failed to get GitHub app installation URL', true)
						}
					}}
				>
					{$enterpriseLicense ? 'Install GitHub app' : 'Install GitHub app (ee only)'}
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
		>{:else}<div class="py-2" />{/if}
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
	<div class="py-2" />
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
