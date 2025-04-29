<script lang="ts">
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { displayDate, copyToClipboard } from '$lib/utils'
	import type { TruncatedToken, NewToken } from '$lib/gen'
	import { UserService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { Clipboard, Plus } from 'lucide-svelte'
	import { workspaceStore, userWorkspaces, type UserWorkspace } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from '../Toggle.svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import { sendUserToast } from '$lib/toast'

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
	let displayCreateToken = $state(scopes != undefined)
	let mcpCreationMode = $state(false)
	let newMcpScope = $state('favorites')
	let newMcpToken = $state<string | undefined>(undefined)

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

	// --- Functions ---
	async function createToken(mcpMode: boolean = false): Promise<void> {
		try {
			let date: Date | undefined
			if (newTokenExpiration) {
				date = new Date(new Date().getTime() + newTokenExpiration * 1000)
			}

			let tokenScopes = mcpMode ? [`mcp:${newMcpScope}`] : scopes

			const createdToken = await UserService.createToken({
				requestBody: {
					label: newTokenLabel,
					expiration: date?.toISOString(),
					scopes: tokenScopes,
					workspace_id: mcpMode ? (newTokenWorkspace || $workspaceStore) : newTokenWorkspace
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
</script>

<div class="grid grid-cols-2 pt-8 pb-1" class:pt-8={scopes == undefined}>
	<h2 class="py-0 my-0 border-b pt-3">Tokens</h2>
	<div class="flex justify-end border-b pb-1 gap-2">
		<Button
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
			class="border rounded-md mb-6 px-2 py-2 bg-green-50 dark:bg-green-200 dark:text-green-800 flex flex-row flex-wrap"
		>
			<p class="text-sm mb-2">New MCP URL:</p>
			<ClipboardPanel content={`${mcpBaseUrl}${newMcpToken}`} />
			<p class="pt-1 text-xs">
				Make sure to copy this URL now. You won't be able to see it again!
			</p>
		</div>
	{/if}

	{#if displayCreateToken}
		<div class="py-3 px-3 border rounded-md mb-6 bg-surface-secondary min-w-min">
			<h3 class="pb-3 font-semibold">Add a new token</h3>
			{#if scopes != undefined}
				{#each scopes as scope}
					<div class="flex flex-col mb-4">
						<label for="label">Scope</label>
						<input disabled type="text" value={scope} />
					</div>
				{/each}
			{/if}
			{#if showMcpMode}
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
					class="mb-4"
					size="xs"
				/>
			{/if}
			<div class="flex flex-row flex-wrap gap-2 w-full justify-between">
				{#if mcpCreationMode}
					<div class="flex flex-col">
						<label for="label">Scope</label>
						<ToggleButtonGroup bind:selected={newMcpScope} allowEmpty={false} let:item>
							<ToggleButton {item} value="favorites" label="Favorites Only" />
							<ToggleButton {item} value="all" label="All Resources" />
						</ToggleButtonGroup>
					</div>
					<div class="flex flex-col">
						<label for="label">Workspace</label>
						<select bind:value={newTokenWorkspace} disabled={workspaces.length === 1}>
							{#each workspaces as workspace}
								<option value={workspace.id}>{workspace.name}</option>
							{/each}
						</select>
					</div>
				{/if}
				<div class="flex flex-col">
					<label for="label">Label <span class="text-xs text-tertiary">(optional)</span></label>
					<input type="text" bind:value={newTokenLabel} />
				</div>
				<div class="flex flex-col">
					<label for="expires"
						>Expires In &nbsp;<span class="text-xs text-tertiary">(optional)</span>
					</label>
					<select bind:value={newTokenExpiration} disabled={mcpCreationMode}>
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
				<div class="flex items-end">
					<Button
						btnClasses="!mt-2"
						on:click={() => createToken(mcpCreationMode)}
						disabled={mcpCreationMode && newTokenWorkspace === undefined}
					>
						New token
					</Button>
				</div>
			</div>
		</div>
	{/if}
</div>

<div class="overflow-auto">
	<TableCustom>
		<tr slot="header-row">
			<th>prefix</th>
			<th>label</th>
			<th>expiration</th>
			<th>scopes</th>
			<th></th>
		</tr>
		<tbody slot="body">
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
