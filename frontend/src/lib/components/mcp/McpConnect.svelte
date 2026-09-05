<script lang="ts">
	import { Button } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Path from '$lib/components/Path.svelte'
	import Password from '$lib/components/Password.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import McpServerOAuthConnect from './McpServerOAuthConnect.svelte'
	import OauthScopes from '$lib/components/OauthScopes.svelte'
	import { sameTopDomainOrigin } from '$lib/cookies'
	import { base } from '$lib/base'
	import { onDestroy, untrack } from 'svelte'
	import { ExternalLink, Pen } from 'lucide-svelte'
	import { MCP_REGISTRY, findMcpEntry, findMcpEntryByUrl } from './registry'
	import { OauthService, ResourceService } from '$lib/gen'
	import { upsertSecretVariable } from './secretVariable'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		/** Carries the workspace the connection was created in, which is not always
		 * the one on screen by the time a popup comes back. */
		onConnected: (workspace: string, resourcePath: string) => void
		/** Omitted where the card is the drawer's own content and there is nothing
		 * to collapse back to. */
		onCancel?: () => void
		/** Required: a caller that forgot it would create the connection in whichever
		 * workspace the ui happens to be showing, not the one it operates on. */
		workspace: string
		/** Off where the surface around it already draws a card — a popover panel —
		 * so the two do not stack a border and a background on each other. */
		bordered?: boolean
	}

	let { onConnected, onCancel, workspace, bordered = true }: Props = $props()

	let ws = $derived(workspace)
	// Any URL is connectable; a suggestion is a shortcut that also pins how the
	// server hands out credentials, which a typed URL cannot tell us.
	let suggested = $state<string | undefined>(undefined)

	let instanceConnects = $state<string[] | undefined>(undefined)
	let signingIn = $state(false)
	let editScopes = $state(false)
	// Seeded from the instance connect, then left editable: a server may want more
	// than the connect asks for (org-scoped search needs read:org, for instance),
	// and the connect itself is shared with other integrations so it is not widened.
	let scopes = $state<string[]>([])
	// The popup url is built from `scopes`, so signing in before the connect's
	// scopes arrive would authorize with none at all and mint a token that cannot
	// reach the tools the user came for.
	let scopesStatus = $state<'loading' | 'loaded' | 'error'>('loading')

	let url = $state('')
	// Discovery is a network call, so it follows the committed url (a picked
	// suggestion, or a typed one on blur) rather than every keystroke.
	let committedUrl = $state('')
	// A pasted url resolves to the same entry as its chip, so github reaches its
	// own connect rather than a discovery its server cannot answer.
	let entry = $derived(suggested ? findMcpEntry(suggested) : findMcpEntryByUrl(committedUrl))
	let manualToken = $state<string | undefined>(undefined)
	let manualPath = $state('')
	let manualPathError = $state('')
	let saving = $state(false)
	let discoveryFoundOAuth = $state<boolean | undefined>(undefined)
	let showToken = $state(false)
	// `u/<me>` is private; a folder or group path means the token travels with the
	// resource to everyone who can read it.
	let sharedPath = $derived(!!manualPath && !manualPath.startsWith(`u/${$userStore?.username}/`))
	let oauthConnect: McpServerOAuthConnect | undefined = $state()
	// The connector owns the popup listener and is keyed on the url, so a url edit
	// or a switch to token entry would destroy it and lose a callback still in
	// flight. Editing waits for the popup instead.
	let oauthPending = $derived(oauthConnect?.isConnecting() ?? false)
	let pending: (Target & { client: string; scopes: string[] }) | undefined = undefined
	let popup: Window | null = null

	/** Slug for the resource value's `name` and for the path suggestion. Hosts are
	 * reduced to the label that names the service, so mcp.notion.com reads notion. */
	let serverName = $derived.by(() => {
		if (entry) return entry.id
		try {
			const labels = new URL(url).hostname.split('.')
			const named = labels.filter((l) => !['www', 'mcp', 'api'].includes(l))
			return (named[0] ?? labels[0] ?? 'mcp').replace(/[^a-z0-9]/gi, '_')
		} catch {
			return 'mcp'
		}
	})
	let suggestedPath = $derived(
		`u/${$userStore?.username ?? 'user'}/${serverName === 'mcp' ? 'mcp_server' : `${serverName}_mcp`}`
	)
	// Path reads `path` only when it mounts, so the suggestion is applied here and
	// the picker is re-keyed on it. A path the user typed is left alone.
	let lastSuggestion = ''
	$effect(() => {
		const next = suggestedPath
		untrack(() => {
			if (manualPath === '' || manualPath === lastSuggestion) manualPath = next
		})
		lastSuggestion = next
	})

	$effect(() => {
		if (instanceConnects === undefined) {
			OauthService.listOauthConnects()
				.then((l) => (instanceConnects = l.map((x) => x.name)))
				.catch(() => (instanceConnects = []))
		}
	})

	// The card asks for a server first and only then for a credential, so nothing
	// below is decided until there is a url to detect against.
	let hasTarget = $derived(!!committedUrl)
	let connectsLoaded = $derived(instanceConnects !== undefined)

	// Which flow this server can actually use here, so the reason a button is
	// missing is visible before it is clicked rather than after it errors. Each
	// waits for the instance connects: answering before they land would offer a
	// token for a server that can sign in a moment later.
	let oauthAppReady = $derived(
		entry?.auth === 'oauth_app' &&
			entry.connectClient !== undefined &&
			(instanceConnects?.includes(entry.connectClient) ?? false)
	)
	let needsOauthApp = $derived(connectsLoaded && entry?.auth === 'oauth_app' && !oauthAppReady)
	let canDiscover = $derived(hasTarget && connectsLoaded && !needsOauthApp && !oauthAppReady)
	// Until the instance connects land we cannot say whether this server can sign
	// in, and offering a token in that gap would answer the question wrongly.
	let awaitingConnects = $derived(hasTarget && !connectsLoaded)
	// A token is the only way in for some servers and a distraction for others, so
	// it is offered outright only once detection says nothing here can sign in.
	let canSignIn = $derived(
		oauthAppReady || (canDiscover && !!$enterpriseLicense && discoveryFoundOAuth !== false)
	)
	// Why the token field is the only way in, said where the token is asked for.
	let tokenNote = $derived(
		needsOauthApp && entry
			? `For an OAuth connection, a superadmin can configure a ${entry.name} OAuth app in the instance settings.`
			: canDiscover && !$enterpriseLicense
				? 'Signing in to an MCP server is an enterprise feature.'
				: undefined
	)

	$effect(() => {
		const client = entry?.connectClient
		if (client && oauthAppReady) {
			scopesStatus = 'loading'
			OauthService.getOauthConnect({ client })
				.then((c) => {
					scopes = c.scopes ?? []
					scopesStatus = 'loaded'
				})
				.catch(() => {
					scopes = []
					scopesStatus = 'error'
				})
		}
	})

	function pick(id: string) {
		if (suggested === id) {
			// Deselecting unlocks the url for editing. The entry keeps applying while
			// the url still points at that server, and drops when it no longer does.
			suggested = undefined
			return
		}
		suggested = id
		url = findMcpEntry(id)?.url ?? url
		committedUrl = url
		discoveryFoundOAuth = undefined
	}

	type Target = {
		workspace: string
		path: string
		url: string
		name: string
		label: string
	}

	/** The server this operation is for, read once: the url field and the
	 * suggestions stay editable while a request or a popup is pending, and the
	 * credential must end up against the server the user aimed at. */
	function target(): Target {
		return {
			workspace: ws,
			path: manualPath,
			url,
			name: serverName,
			label: entry?.name ?? serverName
		}
	}

	async function createMcpResource(t: Target, tokenRef: string) {
		await ResourceService.createResource({
			workspace: t.workspace,
			requestBody: {
				resource_type: 'mcp',
				path: t.path,
				value: { name: t.name, url: t.url, token: tokenRef },
				description: `${t.label} MCP server`
			}
		})
		onConnected(t.workspace, t.path)
	}

	function startProviderOAuth() {
		const client = entry?.connectClient
		if (!client || !manualPath || scopesStatus !== 'loaded') return
		// The popup outlives any change on this page, so what it comes back to must be
		// the server, path, provider and scopes it was opened for. The scope list in
		// particular stays editable behind it, and the account records what the grant
		// was actually asked for: a mismatch there breaks the refresh, not the connect.
		pending = { ...target(), client, scopes: [...scopes] }
		const connectUrl = new URL(`/api/oauth/connect/${client}`, window.location.origin)
		connectUrl.searchParams.set('scopes', pending.scopes.join('+'))
		popup = window.open(connectUrl.toString(), '_blank', 'popup=true')
		if (!popup) {
			pending = undefined
			sendUserToast('Popup blocked. Allow popups for this site.', true)
			return
		}
		window.addEventListener('message', onOAuthMessage)
		window.addEventListener('storage', onOAuthStorage)
		signingIn = true
	}

	function onOAuthMessage(event: MessageEvent) {
		if (!sameTopDomainOrigin(event.origin, window.location.origin)) return
		// The callback page is shared by every connect on this origin, and every open
		// card listens on the same window: without identifying the popup, another
		// card's success would be stored as this server's credential and another
		// card's failure would tear this one down while its own popup is still open.
		if (!pending || event.source !== popup) return
		if (event.data?.type === 'success') {
			if (event.data.resource_type !== pending.client) return
			cleanupOAuth()
			void finishProviderOAuth(event.data.res)
		} else if (event.data?.type === 'error') {
			cleanupOAuth()
			pending = undefined
			popup = null
			signingIn = false
			sendUserToast(event.data.error, true)
		}
	}

	function onOAuthStorage(event: StorageEvent) {
		if (event.key !== 'oauth-callback') return
		try {
			const data = JSON.parse(event.newValue || '{}')
			if (data.type === 'success' && (!pending || data.resource_type !== pending.client)) return
			cleanupOAuth()
			localStorage.removeItem('oauth-callback')
			if (data.type === 'success') {
				void finishProviderOAuth(data.res)
			} else {
				signingIn = false
				sendUserToast(data.error, true)
			}
		} catch (e) {
			signingIn = false
			console.error('Error parsing oauth callback', e)
		}
	}

	function cleanupOAuth() {
		window.removeEventListener('message', onOAuthMessage)
		window.removeEventListener('storage', onOAuthStorage)
	}

	onDestroy(cleanupOAuth)

	/** Store the token like the resource connect does: a secret variable, plus an
	 * account when the provider issues expiring tokens so refresh can run. */
	async function finishProviderOAuth(res: any) {
		const t = pending
		if (!t) return
		const { workspace, path } = t
		try {
			let account: number | undefined = undefined
			if (res?.expires_in != undefined) {
				account = Number(
					await OauthService.createAccount({
						workspace,
						requestBody: {
							refresh_token: res.refresh_token ?? '',
							expires_in: res.expires_in,
							client: t.client,
							scopes: t.scopes
						}
					})
				)
			}
			await upsertSecretVariable({
				workspace,
				path,
				value: res.access_token,
				resourcePath: path,
				isOauth: true,
				account
			})
			await createMcpResource(t, `$var:${path}`)
			sendUserToast(`Connected ${t.label}`)
		} catch (e) {
			sendUserToast(`Failed to connect ${t.label}: ${e.body ?? e.message}`, true)
		} finally {
			pending = undefined
			popup = null
			signingIn = false
		}
	}

	async function saveManual() {
		const token = manualToken
		const t = target()
		if (!t.url || !token || !t.path) return
		saving = true
		try {
			await upsertSecretVariable({
				workspace: t.workspace,
				path: `${t.path}_token`,
				value: token,
				resourcePath: t.path
			})
			await createMcpResource(t, `$var:${t.path}_token`)
			sendUserToast(`Connected ${t.label}`)
		} catch (e) {
			sendUserToast(`Failed to connect: ${e.body ?? e.message}`, true)
		} finally {
			saving = false
		}
	}
</script>

<div
	class={bordered
		? 'border rounded p-4 bg-surface-tertiary flex flex-col gap-4'
		: 'flex flex-col gap-4'}
>
	<div class="flex justify-between items-center">
		<span class="text-sm font-semibold text-emphasis">Connect an MCP server</span>
		{#if onCancel}
			<Button unifiedSize="2xs" variant="subtle" onClick={onCancel}>Cancel</Button>
		{/if}
	</div>

	<Alert type="info" size="xs" title="Only HTTP streamable MCP servers are supported" />

	<div class="flex flex-col gap-1">
		<Label label="MCP server URL">
			{#snippet action()}
				{#if entry?.docsUrl}
					<a
						href={entry.docsUrl}
						target="_blank"
						rel="noopener noreferrer"
						class="text-2xs text-accent hover:underline inline-flex items-center gap-1"
					>
						{entry.name} docs <ExternalLink size={12} />
					</a>
				{/if}
			{/snippet}
			<TextInput
				inputProps={{
					type: 'url',
					placeholder: 'https://mcp.example.com',
					disabled: suggested !== undefined || oauthPending,
					onchange: () => ((committedUrl = url), (discoveryFoundOAuth = undefined))
				}}
				bind:value={url}
			/>
		</Label>
		<div class="flex flex-row flex-wrap items-center gap-1">
			<span class="text-2xs text-secondary mr-1">Suggested</span>
			{#each MCP_REGISTRY as e (e.id)}
				<Button
					unifiedSize="2xs"
					variant="subtle"
					selected={entry?.id === e.id}
					disabled={oauthPending}
					startIcon={{ icon: e.icon, props: { width: '12px', height: '12px' } }}
					onClick={() => pick(e.id)}
				>
					{e.name}
				</Button>
			{/each}
		</div>
	</div>

	{#if hasTarget && !awaitingConnects}
		{#if showToken || !canSignIn}
			<Label label="Token">
				{#if entry?.tokenHint || tokenNote}
					<span class="text-xs text-secondary">
						{entry?.tokenHint ?? ''}
						{tokenNote ?? ''}
					</span>
				{/if}
				<Password bind:password={manualToken} />
			</Label>
		{:else if oauthAppReady && entry}
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis flex gap-2 items-center">
					OAuth scopes
					<Button
						unifiedSize="2xs"
						variant="subtle"
						iconOnly
						title="Edit scopes"
						startIcon={{ icon: Pen }}
						onClick={() => (editScopes = !editScopes)}
					/>
				</span>
				{#if editScopes}
					<OauthScopes bind:scopes />
				{:else}
					<div class="flex flex-col gap-1">
						{#each scopes as scope}
							<div class="py-0.5 pl-2 text-xs">- {scope}</div>
						{/each}
					</div>
				{/if}
			</div>
			{#if scopesStatus === 'error'}
				<div class="text-2xs text-secondary">
					Could not load the scopes for the {entry.name} connect. Reload to try again.
				</div>
			{/if}
		{:else if canDiscover && $enterpriseLicense}
			{#key committedUrl}
				<McpServerOAuthConnect
					bind:this={oauthConnect}
					server={{ name: entry?.name ?? serverName, url: committedUrl }}
					path={manualPath}
					onDiscovered={(supported) => (discoveryFoundOAuth = supported)}
					workspace={ws}
					onConnected={(connectedWorkspace, path) => onConnected(connectedWorkspace, path)}
				/>
			{/key}
		{/if}

		<Label label="Save MCP connection to">
			<!-- The path decides who gets the connection, and its token with it: the backend
			     reads both off the path (`u/<name>` is that user's, `f/<folder>` is everyone
			     with read on the folder), so this is the one place to say so. -->
			<span class="text-xs text-secondary">
				Under <span class="font-mono">u/{$userStore?.username ?? 'you'}</span> it is yours alone; in
				a folder, everyone in it can use the connection and its token. Saved as an
				<a
					href="{base}/resources?workspace={ws}"
					target="_blank"
					rel="noopener noreferrer"
					class="text-accent hover:underline inline-flex items-center gap-1"
				>
					MCP resource <ExternalLink size={12} />
				</a>
			</span>
			{#key suggestedPath}
				<Path
					bind:path={manualPath}
					bind:error={manualPathError}
					initialPath=""
					namePlaceholder={serverName}
					kind="resource"
					workspaceOverride={ws}
				/>
			{/key}
		</Label>
		{#if sharedPath}
			<Alert type="warning" size="xs" title="Anyone who can read this path can use this connection">
				Its tools run against the account the token belongs to.
			</Alert>
		{/if}

		<div class="flex flex-col gap-2">
			{#if showToken || !canSignIn}
				<Button
					unifiedSize="sm"
					variant="accent"
					wrapperClasses="self-start"
					onClick={saveManual}
					disabled={saving || !url || !manualToken || !manualPath || manualPathError !== ''}
				>
					Save
				</Button>
			{:else if oauthAppReady && entry}
				<Button
					unifiedSize="sm"
					variant="accent"
					wrapperClasses="self-start"
					onClick={startProviderOAuth}
					disabled={signingIn || scopesStatus !== 'loaded' || !manualPath || manualPathError !== ''}
				>
					{signingIn ? 'Finish in the popup...' : `Sign in with ${entry.name}`}
				</Button>
			{:else if canDiscover && $enterpriseLicense}
				<Button
					unifiedSize="sm"
					variant="accent"
					wrapperClasses="self-start"
					onClick={() => oauthConnect?.start()}
					disabled={!oauthConnect?.canStart() || !manualPath || manualPathError !== ''}
				>
					{oauthConnect?.isConnecting()
						? 'Finish in the popup...'
						: `Sign in with ${entry?.name ?? serverName}`}
				</Button>
			{/if}

			{#if canSignIn}
				<Button
					unifiedSize="2xs"
					variant="subtle"
					wrapperClasses="self-start"
					disabled={oauthPending}
					onClick={() => (showToken = !showToken)}
				>
					{showToken
						? `Sign in with ${entry?.name ?? serverName} instead`
						: 'Connect with a token instead'}
				</Button>
			{/if}
		</div>
	{/if}
</div>
