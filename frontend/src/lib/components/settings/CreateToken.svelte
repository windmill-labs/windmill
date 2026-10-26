<script lang="ts">
	import { onMount, untrack } from 'svelte'
	import { userWorkspaces, usersWorkspaceStore, type UserWorkspace } from '$lib/stores'
	import { Button, Skeleton } from '../common'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import Toggle from '../Toggle.svelte'
	import { SettingService, UserService, type NewToken } from '$lib/gen'
	import { mcpTokenUrlDisabled } from '$lib/mcpAuth'
	import { apiBaseUrl } from '$lib/apiBaseUrl.svelte'
	import TokenDisplay from './TokenDisplay.svelte'
	import ScopesPicker from './ScopesPicker.svelte'
	import { parseMaxTokenExpirationDays } from '$lib/tokenExpiration'

	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import Section from '../Section.svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import McpAccessPicker, { type McpAccess } from '../mcp/McpAccessPicker.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
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

	let newToken = $state<string | undefined>(undefined)
	let newMcpToken = $state<string | undefined>(undefined)
	let newMcpTokenUrl = $state('')
	// What an MCP token may do, as the access picker sets it.
	let mcpAccess = $state<McpAccess>('full')
	let mcpScopes = $state<string[]>(['mcp:all'])
	// What the user picked; `newTokenExpiration` is that pick held within the ceiling.
	let pickedExpiration = $state<number | undefined>(undefined)
	let newTokenWorkspace = $state<string | undefined>(untrack(() => defaultNewTokenWorkspace))
	// The workspaces an MCP token reaches. One binds the token to it; several leave it unbound
	// and are listed in its scopes; all of them leaves it unbound with no list, so it also
	// reaches the workspaces its owner joins later.
	let mcpWorkspaces = $state<string[]>([])
	let mcpAllWorkspaces = $state(false)
	let mcpCreationMode = $state(false)
	let lastRequestedMcpMode = $state<boolean | undefined>(undefined)
	let mcpLabelAutofilled = $state(false)

	let pickedScopes = $state<string[] | null>(null)
	let readOnly = $state(false)
	// How this instance lets an MCP client in. `oauth` means it refuses `?token=`, so a
	// generated token would not get a client in and the URL is handed over bare instead.
	// A failed read lands on `oauth`: the bare URL works whichever way the setting is,
	// whereas guessing `token` mints a non-expiring credential the server may refuse.
	type McpUrlPolicy = 'loading' | 'token' | 'oauth'
	let mcpUrlPolicy = $state<McpUrlPolicy>('loading')

	async function loadMcpUrlPolicy() {
		mcpUrlPolicy = 'loading'
		try {
			mcpUrlPolicy = (await mcpTokenUrlDisabled()) ? 'oauth' : 'token'
		} catch (err) {
			console.error('Failed to load the MCP token setting:', err)
			mcpUrlPolicy = 'oauth'
		}
	}

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
		void loadMcpUrlPolicy()
		resetExpirationOnModeChange()
		const current = defaultNewTokenWorkspace ?? $operatingWorkspace
		mcpWorkspaces = current ? [current] : []
		mcpAllWorkspaces = false
		newToken = undefined
		newMcpToken = undefined
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

			const pickedOrFixed = scopes ?? (mcpMode ? mcpScopes : pickedScopes) ?? undefined
			const tokenScopes =
				pickedOrFixed && workspacesScope ? [...pickedOrFixed, workspacesScope] : pickedOrFixed

			const createdToken = await UserService.createToken({
				requestBody: {
					label: newTokenLabel,
					expiration: date?.toISOString(),
					scopes: tokenScopes,
					workspace_id: tokenWorkspaceId,
					read_only: mcpMode ? false : readOnly
				} as NewToken
			})

			if (mcpMode) {
				newToken = undefined
				newMcpToken = `${createdToken}`
				newMcpTokenUrl = `${mcpUrl}?token=${createdToken}`
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
	const mcpBoundWorkspace = $derived(
		!mcpAllWorkspaces && mcpWorkspaces.length === 1 ? mcpWorkspaces[0] : undefined
	)
	const mcpReachesWorkspaces = $derived(mcpAllWorkspaces || mcpWorkspaces.length > 0)
	const tokenWorkspaceId = $derived(mcpCreationMode ? mcpBoundWorkspace : newTokenWorkspace)
	const workspacesScope = $derived(
		mcpCreationMode && !mcpAllWorkspaces && mcpWorkspaces.length > 1
			? `mcp:workspaces:${mcpWorkspaces.join(',')}`
			: undefined
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
	// The workspace used to browse scripts/flows/endpoints in the scope picker: the first
	// picked one, or the current one for a token reaching all of them.
	const scopeWorkspaceId = $derived(
		(mcpCreationMode && !mcpAllWorkspaces ? mcpWorkspaces[0] : newTokenWorkspace) ||
			$operatingWorkspace ||
			''
	)
	// The server builds this URL's OAuth metadata from the same API base url, and a client
	// rejects a mismatch. One URL whatever the access: the token's scopes decide which tools it gets.
	const mcpUrl = $derived(`${apiBaseUrl('mcp')}/api/mcp`)
	// Selected scripts and flows are listed per workspace: across several, a path could be two.
	const selectionNeedsOneWorkspace = $derived(
		mcpAccess === 'selected' && (mcpAllWorkspaces || mcpWorkspaces.length !== 1)
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

{#snippet mcpTokenForm()}
	<div class="flex flex-col gap-3">
		<McpAccessPicker
			workspaceId={scopeWorkspaceId}
			bind:scopes={mcpScopes}
			bind:access={mcpAccess}
		/>
		{#if !lockWorkspace}
			{@render mcpWorkspacePicker()}
			{#if selectionNeedsOneWorkspace}
				<p class="text-2xs text-secondary">
					Selected scripts and flows come from one workspace: pick one.
				</p>
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
		{#if maxExpirationSecs != undefined}
			<div>
				<span class="block mb-1 text-xs text-emphasis font-semibold">Expires In</span>
				<Select
					bind:value={() => newTokenExpiration, (v) => (pickedExpiration = v)}
					placeholder="Pick an expiration"
					inputClass="w-full"
					items={expirationItems}
				/>
				<p class="mt-1 text-xs text-tertiary"
					>This instance limits tokens to {maxExpirationLabel}.</p
				>
			</div>
		{/if}
		<div class="flex">
			<Button
				onclick={() => createToken(true)}
				disabled={!mcpReachesWorkspaces || selectionNeedsOneWorkspace || mcpScopes.length === 0}
				variant="default"
				unifiedSize="sm"
			>
				Generate URL with token
			</Button>
		</div>
	</div>
{/snippet}

{#snippet mcpWorkspacePicker()}
	<div class="flex flex-col gap-1">
		<span class="block text-emphasis text-xs font-semibold">Workspaces</span>
		<Toggle
			bind:checked={mcpAllWorkspaces}
			options={{ right: 'All my workspaces, including ones I join later' }}
			size="xs"
		/>
		{#if !mcpAllWorkspaces}
			<MultiSelect
				bind:value={mcpWorkspaces}
				items={workspaces.map((w) => ({ label: w.name, value: w.id }))}
				placeholder="Pick workspaces"
				reorderable={false}
			/>
		{/if}
		{#if mcpAllWorkspaces || mcpWorkspaces.length > 1}
			<p class="text-xs text-tertiary">
				Tools take a <code>workspace_id</code> argument naming the workspace they act in.
			</p>
		{/if}
	</div>
{/snippet}

<div>
	<!-- Stays bounded by the panel width: a content-driven width (min-w-min) would let a long
	     scope chip stretch this card and push the rest of the form out of view. -->
	<div class="p-4 rounded-md bg-surface-tertiary {!mcpOnly || newMcpToken ? 'mb-6' : ''}">
		{#if !mcpOnly}
			<h3 class="pb-2 font-semibold text-emphasis text-sm">{title}</h3>
		{/if}

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

		{#if mcpCreationMode && mcpUrlPolicy === 'loading'}
			<Skeleton layout={[[2], 0.5, [1]]} />
		{:else if mcpCreationMode}
			<div class="flex flex-col gap-6">
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold text-emphasis">MCP URL</span>
					<ClipboardPanel content={mcpUrl} size="sm" />
					<p class="text-2xs text-secondary">
						Add it to your client as a remote MCP server. It opens a Windmill page where you sign in
						and choose the workspaces and what the client may do: build and run, read and run, read
						only, or only the scripts and flows you pick.
					</p>
				</div>

				{#if mcpUrlPolicy === 'token'}
					<Section label="Use a token instead of signing in" small collapsable collapsed>
						{@render mcpTokenForm()}
					</Section>
				{/if}

				{#if !mcpOnly}
					<div class="flex justify-end">
						<Button onClick={exitMcpMode} variant="default">Cancel</Button>
					</div>
				{/if}
			</div>
		{:else}
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
					mode="standard"
					workspaceId={scopeWorkspaceId}
					bind:value={pickedScopes}
					bind:readOnly
				/>
			{/if}

			<div class="mt-2 grid grid-cols-1 md:grid-cols-2 gap-4">
				{#if !mcpOnly}
					<div>
						<span class="block mb-1 text-emphasis text-xs font-semibold"
							>Label <span class="text-xs text-primary">(optional)</span></span
						>
						<TextInput inputProps={{ type: 'text' }} bind:value={newTokenLabel} class="w-full" />
					</div>
				{/if}

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
				<Button on:click={() => createToken()} variant="accent">New token</Button>
			</div>
		{/if}
	</div>

	{#if newToken && displayCreateToken}
		<TokenDisplay token={newToken} />
	{/if}

	{#if newMcpToken && displayCreateToken}
		<TokenDisplay token={newMcpToken} mcpUrl={newMcpTokenUrl} />
	{/if}
</div>
