<script lang="ts">
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { displayDate, copyToClipboard } from '$lib/utils'
	import type { TruncatedToken, NewToken } from '$lib/gen'
	import { IntegrationService, UserService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { Clipboard, Plus } from 'lucide-svelte'
	import { workspaceStore, userWorkspaces, type UserWorkspace } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from '../Toggle.svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import { sendUserToast } from '$lib/toast'
	import TriggerableByAI from '../TriggerableByAI.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'

	// --- Props ---
	interface Props {
		showMcpMode?: boolean
		openWithMcpMode?: boolean
		defaultNewTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
	}

	let {
		showMcpMode = false,
		openWithMcpMode = false,
		defaultNewTokenLabel,
		defaultNewTokenWorkspace,
		scopes
	}: Props = $props()

	// --- Local State ---
	let tokens = $state<TruncatedToken[]>([])
	let tokenPage = $state(1)
	let newTokenLabel = $state<string | undefined>(defaultNewTokenLabel)
	let newToken = $state<string | undefined>(undefined)
	let newTokenExpiration = $state<number | undefined>(undefined)
	let newTokenWorkspace = $state<string | undefined>(defaultNewTokenWorkspace)
	let newMcpApps = $state<string[]>([])
	let displayCreateToken = $state(scopes != undefined)
	let mcpCreationMode = $state(false)
	let newMcpScope = $state('favorites')
	let newMcpToken = $state<string | undefined>(undefined)
	let loadingApps = $state(false)
	let errorFetchApps = $state(false)
	let allApps = $state<string[]>([])

	function ensureCurrentWorkspaceIncluded(
		workspacesList: UserWorkspace[],
		currentWorkspace: string | undefined
	) {
		if (!currentWorkspace) {
			return workspacesList
		}
		const hasCurrentWorkspace = workspacesList.some((w) => w.id === currentWorkspace)
		if (hasCurrentWorkspace) {
			return workspacesList
		}
		return [{ id: currentWorkspace, name: currentWorkspace }, ...workspacesList]
	}

	const workspaces = $derived(ensureCurrentWorkspaceIncluded($userWorkspaces, $workspaceStore))
	const mcpBaseUrl = $derived(`${window.location.origin}/api/mcp/w/${newTokenWorkspace}/sse?token=`)
	const dispatch = createEventDispatcher()

	$effect(() => {
		if (openWithMcpMode) {
			handleCreateClick('mcpUrl')
		}
		listTokens()
	})

	$effect(() => {
		if (mcpCreationMode) {
			getAllApps()
		} else {
			newMcpApps = []
		}
	})

	// --- Functions ---
	async function createToken(mcpMode: boolean = false): Promise<void> {
		try {
			let date: Date | undefined
			if (newTokenExpiration) {
				date = new Date(new Date().getTime() + newTokenExpiration * 1000)
			}

			let tokenScopes = scopes
			if (mcpMode) {
				tokenScopes = [`mcp:${newMcpScope}`]
				if (newMcpApps.length > 0) {
					tokenScopes.push(`mcp:hub:${newMcpApps.join(',')}`)
				}
			}

			const createdToken = await UserService.createToken({
				requestBody: {
					label: newTokenLabel,
					expiration: date?.toISOString(),
					scopes: tokenScopes,
					workspace_id: mcpMode ? newTokenWorkspace || $workspaceStore : newTokenWorkspace
				} as NewToken
			})

			if (mcpMode) {
				newMcpToken = `${createdToken}`
			} else {
				newToken = `${createdToken}`
			}

			dispatch('tokenCreated', newToken ?? newMcpToken)
			listTokens()
			mcpCreationMode = false
			displayCreateToken = false
		} catch (err) {
			console.error('Failed to create token:', err)
		}
	}

	function handleCreateClick(type: 'token' | 'mcpUrl') {
		mcpCreationMode = type === 'mcpUrl'
		displayCreateToken = true
		newMcpToken = undefined
		newToken = undefined
		newTokenExpiration = undefined
		newMcpApps = []
		newTokenLabel = type === 'mcpUrl' ? 'MCP token' : undefined
	}

	function handleCreateTokenClick() {
		handleCreateClick('token')
	}

	function handleCopyClick() {
		copyToClipboard(newToken ?? '')
	}

	async function handleDeleteClick(tokenPrefix: string) {
		await UserService.deleteToken({ tokenPrefix })
		sendUserToast('Successfully deleted token')
		listTokens()
	}

	async function listTokens(): Promise<void> {
		tokens = await UserService.listTokens({
			excludeEphemeral: true,
			page: tokenPage,
			perPage: 100
		})
	}

	function handleNextPage() {
		tokenPage += 1
		listTokens()
	}

	function handlePreviousPage() {
		tokenPage -= 1
		listTokens()
	}

	async function getAllApps() {
		if (allApps.length > 0) {
			return
		}
		try {
			loadingApps = true
			allApps = (
				await IntegrationService.listHubIntegrations({
					kind: 'script'
				})
			).map((x) => x.name)
		} catch (err) {
			console.error('Hub is not available')
			allApps = []
			errorFetchApps = true
		} finally {
			loadingApps = false
		}
	}
</script>

<div class="grid grid-cols-2 pt-8 pb-1" class:pt-8={scopes == undefined}>
	<h2 class="py-0 my-0 border-b pt-3">Tokens</h2>
	<div class="flex justify-end border-b pb-1 gap-2">
		<Button
			aiId="account-settings-create-token"
			aiDescription="Create a new token to authenticate to the Windmill API"
			size="sm"
			startIcon={{ icon: Plus }}
			btnClasses={displayCreateToken ? 'hidden' : ''}
			on:click={handleCreateTokenClick}
		>
			Create token
		</Button>
	</div>
</div>
<div class="text-2xs text-tertiary italic pb-6">
	Authenticate to the Windmill API with access tokens.
</div>

<div>
	{#if newToken}
		<div
			class="border rounded-md mb-6 px-2 py-2 bg-green-50 dark:bg-green-200 dark:text-green-800 flex flex-row flex-wrap"
		>
			<div>
				Added token: <button onclick={handleCopyClick} class="inline-flex gap-2 items-center">
					{newToken}
					<Clipboard size={12} />
				</button>
			</div>
			<div class="pt-1 text-xs ml-2">
				Make sure to copy your personal access token now. You won't be able to see it again!
			</div>
		</div>
	{/if}

	{#if newMcpToken}
		<div
			class="border rounded-md mb-6 px-2 py-2 bg-green-50 dark:bg-green-200 dark:text-green-800 flex flex-col"
		>
			<p class="text-sm mb-2">New MCP URL:</p>
			<ClipboardPanel content={`${mcpBaseUrl}${newMcpToken}`} />
			<p class="pt-1 text-xs">
				Make sure to copy this URL now. You won't be able to see it again!
			</p>
			<div class="flex flex-row justify-end gap-2 items-center">
				<a
					href={`https://cursor.com/install-mcp?name=windmill&config=${btoa(
						JSON.stringify({
							url: `${mcpBaseUrl}${newMcpToken}`
						})
					)}`}
					class="pt-2"
					><img
						src="https://cursor.com/deeplink/mcp-install-dark.svg"
						alt="Add windmill MCP server to Cursor"
						height="32"
					/></a
				>
			</div>
		</div>
	{/if}

	{#if displayCreateToken}
		<div class="py-3 px-3 border rounded-md mb-6 bg-surface-secondary min-w-min">
			<h3 class="pb-3 font-semibold">Add a new token</h3>

			{#if showMcpMode}
				<div class="mb-4 flex flex-row flex-shrink-0">
					<TriggerableByAI
						id="account-settings-create-mcp-token"
						description="Create a new MCP token to authenticate to the Windmill API"
					>
						<Toggle
							on:change={(e) => {
								mcpCreationMode = e.detail
								if (e.detail) {
									newTokenLabel = 'MCP token'
									newTokenExpiration = undefined
									newTokenWorkspace = $workspaceStore
								} else {
									newTokenLabel = undefined
									newTokenExpiration = undefined
									newTokenWorkspace = defaultNewTokenWorkspace
								}
							}}
							checked={mcpCreationMode}
							options={{
								right: 'Generate MCP URL',
								rightTooltip:
									'Generate a new MCP URL to make your scripts and flows available as tools through your LLM clients.',
								rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/mcp'
							}}
							size="xs"
						/>
					</TriggerableByAI>
				</div>
			{/if}

			{#if scopes != undefined}
				<div class="mb-4">
					<span class="block mb-1">Scope</span>
					{#each scopes as scope}
						<input disabled type="text" value={scope} class="mb-2 w-full" />
					{/each}
				</div>
			{/if}

			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#if mcpCreationMode}
					<div>
						<span class="block mb-1">Scope</span>
						<ToggleButtonGroup bind:selected={newMcpScope} allowEmpty={false}>
							{#snippet children({ item })}
								<ToggleButton
									{item}
									value="favorites"
									label="Favorites only"
									tooltip="Make only your favorite scripts and flows available as tools"
								/>
								<ToggleButton
									{item}
									value="all"
									label="All scripts/flows"
									tooltip="Make all your scripts and flows available as tools"
								/>
							{/snippet}
						</ToggleButtonGroup>
					</div>

					<div>
						<span class="block mb-1">Hub scripts (optional)</span>
						{#if loadingApps}
							<div>Loading...</div>
						{:else if errorFetchApps}
							<div>Error fetching apps</div>
						{:else}
							<MultiSelect
								items={safeSelectItems(allApps)}
								placeholder="Select apps"
								bind:value={newMcpApps}
								class="!bg-surface"
							/>
						{/if}
					</div>

					<div>
						<span class="block mb-1">Workspace</span>
						<select
							bind:value={newTokenWorkspace}
							disabled={workspaces.length === 1}
							class="w-full !bg-surface"
						>
							{#each workspaces as workspace}
								<option value={workspace.id}>{workspace.name}</option>
							{/each}
						</select>
					</div>
				{/if}

				<div>
					<span class="block mb-1">Label <span class="text-xs text-tertiary">(optional)</span></span
					>
					<input type="text" bind:value={newTokenLabel} class="w-full !bg-surface" />
				</div>

				{#if !mcpCreationMode}
					<div>
						<span class="block mb-1"
							>Expires In <span class="text-xs text-tertiary">(optional)</span></span
						>
						<select bind:value={newTokenExpiration} class="w-full !bg-surface">
							<option value={undefined}>No expiration</option>
							<option value={15 * 60}>15m</option>
							<option value={30 * 60}>30m</option>
							<option value={1 * 60 * 60}>1h</option>
							<option value={1 * 24 * 60 * 60}>1d</option>
							<option value={7 * 24 * 60 * 60}>7d</option>
							<option value={30 * 24 * 60 * 60}>30d</option>
							<option value={90 * 24 * 60 * 60}>90d</option>
						</select>
					</div>
				{/if}
			</div>

			<div class="mt-4 flex justify-end gap-2 flex-row">
				<Button
					on:click={() => {
						mcpCreationMode = false
						displayCreateToken = false
					}}
				>
					Cancel
				</Button>
				<Button
					on:click={() => createToken(mcpCreationMode)}
					disabled={mcpCreationMode && newTokenWorkspace == undefined}
				>
					New token
				</Button>
			</div>
		</div>
	{/if}
</div>

<div class="overflow-auto">
	<TableCustom>
		<!-- @migration-task: migrate this slot by hand, `header-row` is an invalid identifier -->
		<tr slot="header-row">
			<th>prefix</th>
			<th>label</th>
			<th>expiration</th>
			<th>scopes</th>
			<th></th>
		</tr>
		{#snippet body()}
			<tbody>
				{#if tokens && tokens.length > 0}
					{#each tokens as { token_prefix, expiration, label, scopes }}
						<tr>
							<td class="grow">{token_prefix}****</td>
							<td class="grow">{label ?? ''}</td>
							<td class="grow">{displayDate(expiration ?? '')}</td>
							<td class="grow">{scopes?.join(', ') ?? ''}</td>
							<td class="grow">
								<button
									class="text-red-500 text-xs underline"
									onclick={() => handleDeleteClick(token_prefix)}
								>
									Delete
								</button>
							</td>
						</tr>
					{/each}
				{:else if tokens && tokens.length === 0}
					<tr class="px-6">
						<td class="text-secondary italic text-xs"> There are no tokens yet</td>
					</tr>
				{:else}
					<tr><td>Loading...</td></tr>
				{/if}
			</tbody>
		{/snippet}
	</TableCustom>
	<div class="flex flex-row-reverse gap-2 w-full">
		{#if tokens?.length == 100}
			<button class="p-1 underline text-sm whitespace-nowrap text-center" onclick={handleNextPage}>
				Next
			</button>
		{/if}
		{#if tokenPage > 1}
			<button
				class="p-1 underline text-sm whitespace-nowrap text-center"
				onclick={handlePreviousPage}
			>
				Previous
			</button>
		{/if}
	</div>
</div>
