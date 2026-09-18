<script lang="ts">
	import { onMount, untrack } from 'svelte'
	import { userWorkspaces, usersWorkspaceStore, type UserWorkspace } from '$lib/stores'
	import { Button } from '../common'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import Toggle from '../Toggle.svelte'
	import { SettingService, UserService, type NewToken } from '$lib/gen'
	import TokenDisplay from './TokenDisplay.svelte'
	import ScopesPicker from './ScopesPicker.svelte'
	import { parseMaxTokenExpirationDays } from '$lib/tokenExpiration'

	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

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
	// What the user picked; `newTokenExpiration` is that pick held within the ceiling.
	let pickedExpiration = $state<number | undefined>(undefined)
	let newTokenWorkspace = $state<string | undefined>(untrack(() => defaultNewTokenWorkspace))
	let mcpCreationMode = $state(false)
	let lastRequestedMcpMode = $state<boolean | undefined>(undefined)
	let mcpLabelAutofilled = $state(false)

	let pickedScopes = $state<string[] | null>(null)
	let readOnly = $state(false)

	const DAY_SECS = 24 * 60 * 60
	const EXPIRATION_CHOICES = [
		{ label: '15 minutes', value: 15 * 60 },
		{ label: '30 minutes', value: 30 * 60 },
		{ label: '1 hour', value: 60 * 60 },
		{ label: '1 day', value: DAY_SECS },
		{ label: '7 days', value: 7 * DAY_SECS },
		{ label: '30 days', value: 30 * DAY_SECS },
		{ label: '90 days', value: 90 * DAY_SECS },
		{ label: '180 days', value: 180 * DAY_SECS },
		{ label: '365 days', value: 365 * DAY_SECS }
	]

	// The `max_token_expiration_days` instance setting. The server shortens any token that asks
	// for longer, or for no expiration, so with it set the form only offers what would be kept.
	let maxExpirationDays = $state<number | undefined>(undefined)

	onMount(async () => {
		let value: unknown
		try {
			value = await SettingService.getGlobal({ key: 'max_token_expiration_days' })
		} catch {
			// The server still shortens tokens itself; the form just offers every choice.
			return
		}
		const days = parseMaxTokenExpirationDays(value)
		if (days == undefined) {
			return
		}
		maxExpirationDays = days
	})

	// Without a ceiling the expiration field is hidden in MCP mode, so a value picked on one side
	// of the toggle must not ride along unseen. With one, the field shows in both modes and keeps
	// its value.
	function resetExpirationOnModeChange() {
		if (maxExpirationSecs == undefined) {
			pickedExpiration = undefined
		}
	}

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
		resetExpirationOnModeChange()
		newTokenWorkspace = defaultNewTokenWorkspace ?? $operatingWorkspace
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
		resetExpirationOnModeChange()
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

			const createdToken = await UserService.createToken({
				requestBody: {
					label: newTokenLabel,
					expiration: date?.toISOString(),
					scopes: tokenScopes,
					workspace_id: tokenWorkspaceId,
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

	const workspaces = $derived(ensureCurrentWorkspaceIncluded($userWorkspaces, $operatingWorkspace))
	const isAllWorkspaces = $derived(newTokenWorkspace === ALL_WORKSPACES)
	const tokenWorkspaceId = $derived(
		isAllWorkspaces
			? undefined
			: mcpCreationMode
				? newTokenWorkspace || $operatingWorkspace
				: newTokenWorkspace
	)

	// Mirrors the server's exemption for tokens owned by a service account: one in the workspace
	// the token is for, or in any workspace for a workspace-less token.
	const isServiceAccount = $derived.by(() => {
		const memberships = $usersWorkspaceStore?.workspaces ?? []
		return tokenWorkspaceId == undefined
			? memberships.some((w) => w.is_service_account)
			: memberships.some((w) => w.id === tokenWorkspaceId && w.is_service_account)
	})
	const maxExpirationSecs = $derived(
		maxExpirationDays == undefined || isServiceAccount ? undefined : maxExpirationDays * DAY_SECS
	)
	const maxExpirationLabel = $derived(
		maxExpirationDays === 1 ? '1 day' : `${maxExpirationDays} days`
	)
	const expirationItems = $derived(
		maxExpirationSecs == undefined
			? [{ label: 'No expiration', value: undefined }, ...EXPIRATION_CHOICES]
			: [
					...EXPIRATION_CHOICES.filter((choice) => choice.value < maxExpirationSecs),
					{ label: `${maxExpirationLabel} (maximum)`, value: maxExpirationSecs }
				]
	)
	// A ceiling can start applying after the pick: once the setting loads, or when the MCP workspace
	// moves to one where the account is not a service account.
	const newTokenExpiration = $derived(
		maxExpirationSecs != undefined &&
			(pickedExpiration == undefined || pickedExpiration > maxExpirationSecs)
			? maxExpirationSecs
			: pickedExpiration
	)
	// The workspace used to browse scripts/flows/endpoints in the scope picker.
	// For an all-workspaces token there is no single workspace, so fall back to
	// the current one just for populating the endpoint list.
	const scopeWorkspaceId = $derived(
		isAllWorkspaces ? $operatingWorkspace || '' : newTokenWorkspace || $operatingWorkspace || ''
	)
	const mcpBaseUrl = $derived(
		isAllWorkspaces
			? `${window.location.origin}/api/mcp/gateway?token=`
			: `${window.location.origin}/api/mcp/w/${newTokenWorkspace}/mcp?token=`
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
			{/if}

			{#if !mcpOnly}
				<div>
					<span class="block mb-1 text-emphasis text-xs font-semibold"
						>Label <span class="text-xs text-primary">(optional)</span></span
					>
					<TextInput inputProps={{ type: 'text' }} bind:value={newTokenLabel} class="w-full" />
				</div>
			{/if}

			{#if !mcpCreationMode || maxExpirationSecs != undefined}
				<div>
					<span class="block mb-1 text-xs text-emphasis font-semibold">
						Expires In
						{#if maxExpirationSecs == undefined}
							<span class="text-xs text-primary">(optional)</span>
						{/if}
					</span>
					<Select
						bind:value={() => newTokenExpiration, (v) => (pickedExpiration = v)}
						placeholder={maxExpirationSecs == undefined ? 'No expiration' : 'Pick an expiration'}
						inputClass="w-full"
						items={expirationItems}
					/>
					{#if maxExpirationSecs != undefined}
						<p class="mt-1 text-xs text-tertiary">
							This instance limits tokens to {maxExpirationLabel}.
						</p>
					{/if}
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
		<TokenDisplay token={newMcpToken} mcpUrl={`${mcpBaseUrl}${newMcpToken}`} />
	{/if}
</div>
