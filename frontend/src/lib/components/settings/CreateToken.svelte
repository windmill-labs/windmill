<script lang="ts">
	import { untrack } from 'svelte'
	import { userWorkspaces, workspaceStore, type UserWorkspace } from '$lib/stores'
	import { Button } from '../common'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import Toggle from '../Toggle.svelte'
	import { UserService, type NewToken } from '$lib/gen'
	import TokenDisplay from './TokenDisplay.svelte'
	import ScopeSelector from './ScopeSelector.svelte'
	import McpScopeSelector from '../mcp/McpScopeSelector.svelte'

	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'

	interface Props {
		showMcpMode?: boolean
		openWithMcpMode?: boolean
		newTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
		onTokenCreated: (token: string) => void
		displayCreateToken?: boolean
	}

	let {
		showMcpMode = false,
		defaultNewTokenWorkspace,
		scopes,
		onTokenCreated,
		newTokenLabel = $bindable(undefined),
		displayCreateToken = true
	}: Props = $props()

	let newToken = $state<string | undefined>(undefined)
	let newMcpToken = $state<string | undefined>(undefined)
	let newTokenExpiration = $state<number | undefined>(undefined)
	let newTokenWorkspace = $state<string | undefined>(untrack(() => defaultNewTokenWorkspace))
	let mcpCreationMode = $state(false)
	let mcpScope = $state('mcp:favorites')

	let customScopes = $state<string[]>([])
	let showCustomScopes = $state(false)

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
				tokenScopes = mcpScope.split(' ').filter((s) => s.length > 0)
			} else if (showCustomScopes && customScopes.length > 0) {
				tokenScopes = customScopes
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

			onTokenCreated(newToken ?? newMcpToken ?? '')
			mcpCreationMode = false
		} catch (err) {
			console.error('Failed to create token:', err)
		}
	}

	const workspaces = $derived(ensureCurrentWorkspaceIncluded($userWorkspaces, $workspaceStore))
	const mcpBaseUrl = $derived(`${window.location.origin}/api/mcp/w/${newTokenWorkspace}/mcp?token=`)
</script>

<div>
	<div class="p-4 rounded-md mb-6 min-w-min bg-surface-tertiary">
		<h3 class="pb-2 font-semibold text-emphasis text-sm">Add a new token</h3>

		{#if showMcpMode}
			<div
				class="mb-4 flex flex-row flex-shrink-0"
				use:triggerableByAI={{
					id: 'account-settings-create-mcp-token',
					description: 'Create a new MCP token to authenticate to the Windmill API'
				}}
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
							'Generate a new MCP URL to make your scripts, flows, and API endpoints available as tools through your LLM clients.',
						rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/mcp'
					}}
					size="xs"
				/>
			</div>
		{/if}

		{#if scopes != undefined}
			<div class="mb-4">
				<span class="block mb-1 text-emphasis text-xs font-semibold">Scope</span>
				{#each scopes as scope (scope)}
					<TextInput inputProps={{ disabled: true }} value={scope} class="mb-2 w-full" />
				{/each}
			</div>
		{/if}

		{#if !mcpCreationMode && (!scopes || scopes.length === 0)}
			<div class="flex flex-col gap-2">
				<Toggle
					checked={showCustomScopes}
					on:change={(e) => {
						showCustomScopes = e.detail
					}}
					options={{
						right: 'Limit token permissions',
						rightTooltip:
							'By default, tokens have full API access. Enable this to restrict the token to specific scopes.'
					}}
					size="xs"
				/>
				{#if showCustomScopes}
					<ScopeSelector bind:selectedScopes={customScopes} />
				{/if}
			</div>
		{/if}

		<div class="mt-2 grid grid-cols-1 md:grid-cols-2 gap-4">
			{#if mcpCreationMode}
				<div class="col-span-2">
					<McpScopeSelector
						workspaceId={newTokenWorkspace || $workspaceStore || ''}
						bind:scope={mcpScope}
					/>
				</div>

				<div>
					<span class="block mb-1 text-emphasis text-xs font-semibold">Workspace</span>
					<Select
						bind:value={newTokenWorkspace}
						items={workspaces.map((w) => ({ label: w.name, value: w.id, subtitle: w.id }))}
					/>
				</div>
			{/if}

			<div>
				<span class="block mb-1 text-emphasis text-xs font-semibold"
					>Label <span class="text-xs text-primary">(optional)</span></span
				>
				<TextInput inputProps={{ type: 'text' }} bind:value={newTokenLabel} class="w-full" />
			</div>

			{#if !mcpCreationMode}
				<div>
					<span class="block mb-1 text-xs text-emphasis font-semibold"
						>Expires In <span class="text-xs text-primary">(optional)</span></span
					>
					<Select
						bind:value={newTokenExpiration}
						placeholder="No expiration"
						inputClass="w-full"
						items={[
							{ label: 'No expiration', value: undefined },
							{ label: '15 minutes', value: 15 * 60 },
							{ label: '30 minutes', value: 30 * 60 },
							{ label: '1 hour', value: 1 * 60 * 60 },
							{ label: '1 day', value: 1 * 24 * 60 * 60 },
							{ label: '7 days', value: 7 * 24 * 60 * 60 },
							{ label: '30 days', value: 30 * 24 * 60 * 60 },
							{ label: '90 days', value: 90 * 24 * 60 * 60 }
						]}
					/>
				</div>
			{/if}
		</div>

		<div class="mt-4 flex justify-end gap-2 flex-row">
			<Button
				on:click={() => {
					mcpCreationMode = false
				}}
				variant="default"
			>
				Cancel
			</Button>
			<Button
				on:click={() => createToken(mcpCreationMode)}
				disabled={mcpCreationMode &&
					(newTokenWorkspace == undefined || !mcpScope || mcpScope.trim().length === 0)}
				variant="accent"
			>
				New token
			</Button>
		</div>
	</div>

	{#if newToken && displayCreateToken}
		<TokenDisplay token={newToken} />
	{/if}

	{#if newMcpToken && displayCreateToken}
		<TokenDisplay token={newMcpToken} mcpUrl={`${mcpBaseUrl}${newMcpToken}`} />
	{/if}
</div>
