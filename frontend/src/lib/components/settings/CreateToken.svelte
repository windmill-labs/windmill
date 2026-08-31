<script lang="ts">
	import { untrack } from 'svelte'
	import { userWorkspaces, workspaceStore, type UserWorkspace } from '$lib/stores'
	import { Button } from '../common'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import Toggle from '../Toggle.svelte'
	import { UserService, type NewToken } from '$lib/gen'
	import TokenDisplay from './TokenDisplay.svelte'
	import ScopesPicker from './ScopesPicker.svelte'

	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'

	interface Props {
		showMcpMode?: boolean
		openWithMcpMode?: boolean
		mcpOnly?: boolean
		lockWorkspace?: boolean
		title?: string
		newTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
		onTokenCreated: (token: string) => void
		displayCreateToken?: boolean
	}

	let {
		showMcpMode = false,
		openWithMcpMode = false,
		mcpOnly = false,
		lockWorkspace = false,
		title = 'Add a new token',
		defaultNewTokenWorkspace,
		scopes,
		onTokenCreated,
		newTokenLabel = $bindable(undefined),
		displayCreateToken = true
	}: Props = $props()

	// Sentinel workspace value meaning "all workspaces the user can access".
	// Produces a workspace-less MCP token served through the /api/mcp/gateway
	// endpoint, where tools take an explicit workspace_id argument.
	const ALL_WORKSPACES = '*'

	let newToken = $state<string | undefined>(undefined)
	let newMcpToken = $state<string | undefined>(undefined)
	let newTokenExpiration = $state<number | undefined>(undefined)
	let newTokenWorkspace = $state<string | undefined>(untrack(() => defaultNewTokenWorkspace))
	let mcpCreationMode = $state(false)
	let lastRequestedMcpMode = $state<boolean | undefined>(undefined)
	let mcpLabelAutofilled = $state(false)

	let pickedScopes = $state<string[] | null>(null)
	let readOnly = $state(false)
	let includeHeaders = $state('')

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

	function enterMcpMode() {
		mcpCreationMode = true
		newTokenExpiration = undefined
		includeHeaders = ''
		newTokenWorkspace = defaultNewTokenWorkspace ?? $workspaceStore
		newToken = undefined
		newMcpToken = undefined
		readOnly = false
		if (!newTokenLabel) {
			newTokenLabel = 'MCP token'
			mcpLabelAutofilled = true
		} else {
			mcpLabelAutofilled = false
		}
	}

	function exitMcpMode() {
		mcpCreationMode = false
		newTokenExpiration = undefined
		includeHeaders = ''
		newTokenWorkspace = defaultNewTokenWorkspace
		newMcpToken = undefined
		readOnly = false
		if (mcpLabelAutofilled) {
			newTokenLabel = undefined
		}
		mcpLabelAutofilled = false
	}

	async function createToken(mcpMode: boolean = false): Promise<void> {
		try {
			let date: Date | undefined
			if (newTokenExpiration) {
				date = new Date(new Date().getTime() + newTokenExpiration * 1000)
			}

			const tokenScopes = scopes ?? pickedScopes ?? undefined

			const workspaceId = isAllWorkspaces
				? undefined
				: mcpMode
					? newTokenWorkspace || $workspaceStore
					: newTokenWorkspace

			const createdToken = await UserService.createToken({
				requestBody: {
					label: newTokenLabel,
					expiration: date?.toISOString(),
					scopes: tokenScopes,
					workspace_id: workspaceId,
					read_only: readOnly
				} as NewToken
			})

			if (mcpMode) {
				newToken = undefined
				newMcpToken = `${createdToken}`
			} else {
				newMcpToken = undefined
				newToken = `${createdToken}`
			}

			onTokenCreated(`${createdToken}`)
			if (!mcpOnly) {
				mcpCreationMode = false
			}
		} catch (err) {
			console.error('Failed to create token:', err)
		}
	}

	const workspaces = $derived(ensureCurrentWorkspaceIncluded($userWorkspaces, $workspaceStore))
	const isAllWorkspaces = $derived(newTokenWorkspace === ALL_WORKSPACES)
	// The workspace used to browse scripts/flows/endpoints in the scope picker.
	// For an all-workspaces token there is no single workspace, so fall back to
	// the current one just for populating the endpoint list.
	const scopeWorkspaceId = $derived(
		isAllWorkspaces ? $workspaceStore || '' : newTokenWorkspace || $workspaceStore || ''
	)
	const mcpBaseUrl = $derived(
		isAllWorkspaces
			? `${window.location.origin}/api/mcp/gateway?token=`
			: `${window.location.origin}/api/mcp/w/${newTokenWorkspace}/mcp?token=`
	)
	// Rides the URL rather than the token so the allowlist is fixed by whoever
	// configures the MCP client, out of reach of the model driving the session.
	// Gated on the workspace too, not just the input: the gateway path publishes no
	// per-runnable tools, so a URL carrying this would promise a guarantee that
	// does not hold there.
	const mcpIncludeHeaderParam = $derived(
		!isAllWorkspaces && includeHeaders.trim()
			? `&include_header=${encodeURIComponent(includeHeaders.trim())}`
			: ''
	)

	$effect(() => {
		const requestedMcpMode = mcpOnly || openWithMcpMode
		if (requestedMcpMode === lastRequestedMcpMode) {
			return
		}

		if (requestedMcpMode) {
			enterMcpMode()
		} else {
			exitMcpMode()
		}

		lastRequestedMcpMode = requestedMcpMode
	})

	$effect(() => {
		if (mcpLabelAutofilled && newTokenLabel !== 'MCP token') {
			mcpLabelAutofilled = false
		}
	})
</script>

<div>
	<!-- Stays bounded by the panel width: a content-driven width (min-w-min) would let a long
	     scope chip stretch this card and push the rest of the form out of view. -->
	<div class="p-4 rounded-md mb-6 bg-surface-tertiary">
		<h3 class="pb-2 font-semibold text-emphasis text-sm">{title}</h3>

		{#if showMcpMode && !mcpOnly}
			<div
				class="mb-4 flex flex-row flex-shrink-0"
				use:triggerableByAI={{
					id: 'account-settings-create-mcp-token',
					description: 'Create a new MCP token to authenticate to the Windmill API'
				}}
			>
				<Toggle
					on:change={(e) => {
						if (e.detail) {
							enterMcpMode()
						} else {
							exitMcpMode()
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
				<div class="text-tertiary">
					<Toggle
						bind:checked={readOnly}
						options={{
							right: 'Read-only',
							rightTooltip:
								'Restricts this token to GET/HEAD endpoints. Any mutating request (POST/PUT/PATCH/DELETE) or job-run action will be rejected with 403, regardless of the scopes listed above.'
						}}
						size="2xs"
					/>
				</div>
			</div>
		{/if}

		{#if !scopes || scopes.length === 0}
			<ScopesPicker
				mode={mcpCreationMode ? 'mcp' : 'standard'}
				workspaceId={scopeWorkspaceId}
				bind:value={pickedScopes}
				bind:readOnly
			/>
		{/if}

		<div class="mt-2 grid grid-cols-1 md:grid-cols-2 gap-4">
			{#if mcpCreationMode}
				{#if !lockWorkspace}
					<div>
						<span class="block mb-1 text-emphasis text-xs font-semibold">Workspace</span>
						<Select
							bind:value={newTokenWorkspace}
							items={[
								{
									label: 'All workspaces',
									value: ALL_WORKSPACES,
									subtitle: 'Multi-workspace'
								},
								...workspaces.map((w) => ({ label: w.name, value: w.id, subtitle: w.id }))
							]}
						/>
						{#if isAllWorkspaces}
							<p class="mt-1 text-xs text-tertiary">
								This token works across every workspace you can access. Tools take a
								<code>workspace_id</code> argument; call <code>list_workspaces</code> to discover them.
							</p>
						{/if}
					</div>
				{/if}

				{#if !isAllWorkspaces}
					<div>
						<span class="block mb-1 text-emphasis text-xs font-semibold"
							>Forward request headers <span class="text-xs text-primary">(optional)</span></span
						>
						<TextInput
							inputProps={{ type: 'text', placeholder: 'x-user-id, x-tenant' }}
							bind:value={includeHeaders}
							class="w-full"
						/>
						<p class="mt-1 text-xs text-tertiary">
							Header names your MCP client sends, comma separated. Each reaches the script as a
							parameter of the same name (<code>X-User-Id</code> becomes <code>x_user_id</code>) and
							is hidden from the tool schema, so the model cannot set it.
						</p>
					</div>
				{/if}
			{/if}

			{#if !mcpOnly}
				<div>
					<span class="block mb-1 text-emphasis text-xs font-semibold"
						>Label <span class="text-xs text-primary">(optional)</span></span
					>
					<TextInput inputProps={{ type: 'text' }} bind:value={newTokenLabel} class="w-full" />
				</div>
			{/if}

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
			{#if !mcpOnly}
				<Button
					on:click={() => {
						exitMcpMode()
					}}
					variant="default"
				>
					Cancel
				</Button>
			{/if}
			<Button
				on:click={() => createToken(mcpCreationMode)}
				disabled={mcpCreationMode && (newTokenWorkspace == undefined || !pickedScopes)}
				variant="accent"
			>
				{mcpCreationMode ? 'Generate MCP URL' : 'New token'}
			</Button>
		</div>
	</div>

	{#if newToken && displayCreateToken}
		<TokenDisplay token={newToken} />
	{/if}

	{#if newMcpToken && displayCreateToken}
		<TokenDisplay
			token={newMcpToken}
			mcpUrl={`${mcpBaseUrl}${newMcpToken}${mcpIncludeHeaderParam}`}
		/>
	{/if}
</div>
