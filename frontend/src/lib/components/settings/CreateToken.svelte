<script lang="ts">
	import { userWorkspaces, workspaceStore, type UserWorkspace } from '$lib/stores'
	import { Button } from '../common'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import TriggerableByAI from '../TriggerableByAI.svelte'
	import Toggle from '../Toggle.svelte'
	import { UserService, type NewToken } from '$lib/gen'
	import { copyToClipboard } from '$lib/utils'
	import { Clipboard } from 'lucide-svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import { createEventDispatcher } from 'svelte'
	import MultiSelectLegacyWrapper from '../multiselect/MultiSelectLegacyWrapper.svelte'

	interface Props {
		showMcpMode?: boolean
		openWithMcpMode?: boolean
		newTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
		actions?: () => void
		displayCreateToken?: boolean
	}

	let {
		showMcpMode = false,
		defaultNewTokenWorkspace,
		scopes,
		actions,
		newTokenLabel = $bindable(undefined)
	}: Props = $props()

	let newToken = $state<string | undefined>(undefined)
	let newMcpToken = $state<string | undefined>(undefined)
	let newTokenExpiration = $state<number | undefined>(undefined)
	let newTokenWorkspace = $state<string | undefined>(defaultNewTokenWorkspace)
	let newMcpApps = $state<string[]>([])
	let mcpCreationMode = $state(false)
	let newMcpScope = $state('favorites')
	let loadingApps = $state(false)
	let errorFetchApps = $state(false)
	let allApps = $state<string[]>([])

	function handleCopyClick() {
		copyToClipboard(newToken ?? '')
	}
	const dispatch = createEventDispatcher()
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
			if (actions) {
				actions()
			}
			mcpCreationMode = false
		} catch (err) {
			console.error('Failed to create token:', err)
		}
	}

	const workspaces = $derived(ensureCurrentWorkspaceIncluded($userWorkspaces, $workspaceStore))
	const mcpBaseUrl = $derived(`${window.location.origin}/api/mcp/w/${newTokenWorkspace}/sse?token=`)
</script>

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
			class="border rounded-md mb-6 px-2 py-2 bg-green-50 dark:bg-green-200 dark:text-green-800 flex flex-row flex-wrap"
		>
			<p class="text-sm mb-2">New MCP URL:</p>
			<ClipboardPanel content={`${mcpBaseUrl}${newMcpToken}`} />
			<p class="pt-1 text-xs">
				Make sure to copy this URL now. You won't be able to see it again!
			</p>
		</div>
	{/if}

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
						<MultiSelectLegacyWrapper
							items={allApps}
							placeholder="Select apps"
							bind:value={newMcpApps}
						/>
					{/if}
				</div>

				<div>
					<span class="block mb-1">Workspace</span>
					<select bind:value={newTokenWorkspace} disabled={workspaces.length === 1} class="w-full">
						{#each workspaces as workspace}
							<option value={workspace.id}>{workspace.name}</option>
						{/each}
					</select>
				</div>
			{/if}

			<div>
				<span class="block mb-1">Label <span class="text-xs text-tertiary">(optional)</span></span>
				<input type="text" bind:value={newTokenLabel} class="w-full" />
			</div>

			{#if !mcpCreationMode}
				<div>
					<span class="block mb-1"
						>Expires In <span class="text-xs text-tertiary">(optional)</span></span
					>
					<select bind:value={newTokenExpiration} class="w-full">
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
</div>
