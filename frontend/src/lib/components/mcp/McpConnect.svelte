<script lang="ts">
	import { Button } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Path from '$lib/components/Path.svelte'
	import Password from '$lib/components/Password.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import McpOAuthConnect from './McpOAuthConnect.svelte'
	import OauthScopes from '$lib/components/OauthScopes.svelte'
	import { sameTopDomainOrigin } from '$lib/cookies'
	import { onDestroy, untrack } from 'svelte'
	import { ExternalLink, Pen } from 'lucide-svelte'
	import { MCP_REGISTRY, findMcpEntry } from './registry'
	import { OauthService, ResourceService } from '$lib/gen'
	import { upsertSecretVariable } from './secretVariable'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		onConnected: (resourcePath: string) => void
		onCancel: () => void
		workspace?: string
	}

	let { onConnected, onCancel, workspace }: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore!)
	// Any URL is connectable; a suggestion is a shortcut that also pins how the
	// server hands out credentials, which a typed URL cannot tell us.
	let suggested = $state<string | undefined>(undefined)
	let entry = $derived(suggested ? findMcpEntry(suggested) : undefined)

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
	let manualToken = $state<string | undefined>(undefined)
	let manualPath = $state('')
	let manualPathError = $state('')
	let saving = $state(false)
	let discoveryFoundOAuth = $state<boolean | undefined>(undefined)
	let showToken = $state(false)
	// `u/<me>` is private; a folder or group path means the token travels with the
	// resource to everyone who can read it.
	let sharedPath = $derived(!!manualPath && !manualPath.startsWith(`u/${$userStore?.username}/`))
	let oauthConnect: McpOAuthConnect | undefined = $state()

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
			// Deselecting leaves the url in place to edit; an edited url is no longer
			// that suggestion, so its auth kind must stop applying with it.
			suggested = undefined
			return
		}
		suggested = id
		url = findMcpEntry(id)?.url ?? url
		committedUrl = url
		discoveryFoundOAuth = undefined
	}

	async function createMcpResource(path: string, tokenRef: string) {
		await ResourceService.createResource({
			workspace: ws,
			requestBody: {
				resource_type: 'mcp',
				path,
				value: { name: serverName, url, token: tokenRef },
				description: `${entry?.name ?? serverName} MCP server`
			}
		})
		onConnected(path)
	}

	function startProviderOAuth() {
		const client = entry?.connectClient
		if (!client || !manualPath || scopesStatus !== 'loaded') return
		const connectUrl = new URL(`/api/oauth/connect/${client}`, window.location.origin)
		connectUrl.searchParams.set('scopes', scopes.join('+'))
		if (!window.open(connectUrl.toString(), '_blank', 'popup=true')) {
			sendUserToast('Popup blocked. Allow popups for this site.', true)
			return
		}
		window.addEventListener('message', onOAuthMessage)
		window.addEventListener('storage', onOAuthStorage)
		signingIn = true
	}

	function onOAuthMessage(event: MessageEvent) {
		if (!sameTopDomainOrigin(event.origin, window.location.origin)) return
		if (event.data?.type === 'success') {
			cleanupOAuth()
			void finishProviderOAuth(event.data.res)
		} else if (event.data?.type === 'error') {
			cleanupOAuth()
			signingIn = false
			sendUserToast(event.data.error, true)
		}
	}

	function onOAuthStorage(event: StorageEvent) {
		if (event.key !== 'oauth-callback') return
		cleanupOAuth()
		try {
			const data = JSON.parse(event.newValue || '{}')
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
		try {
			let account: number | undefined = undefined
			if (res?.expires_in != undefined) {
				account = Number(
					await OauthService.createAccount({
						workspace: ws,
						requestBody: {
							refresh_token: res.refresh_token ?? '',
							expires_in: res.expires_in,
							client: entry!.connectClient!,
							scopes
						}
					})
				)
			}
			await upsertSecretVariable({
				workspace: ws,
				path: manualPath,
				value: res.access_token,
				isOauth: true,
				account,
				description: `OAuth token for ${entry!.name}`
			})
			await createMcpResource(manualPath, `$var:${manualPath}`)
			sendUserToast(`Connected ${entry!.name}`)
		} catch (e) {
			sendUserToast(`Failed to connect ${entry?.name}: ${e.body ?? e.message}`, true)
		} finally {
			signingIn = false
		}
	}

	async function saveManual() {
		const token = manualToken
		if (!url || !token || !manualPath) return
		saving = true
		try {
			await upsertSecretVariable({
				workspace: ws,
				path: `${manualPath}_token`,
				value: token,
				description: `Token for the ${manualPath} MCP server`
			})
			await createMcpResource(manualPath, `$var:${manualPath}_token`)
			sendUserToast(`Connected ${entry?.name ?? url}`)
		} catch (e) {
			sendUserToast(`Failed to connect: ${e.body ?? e.message}`, true)
		} finally {
			saving = false
		}
	}
</script>

<div class="border rounded p-4 bg-surface-secondary flex flex-col gap-4">
	<div class="flex justify-between items-center">
		<span class="text-sm font-semibold text-emphasis">Connect an MCP server</span>
		<Button unifiedSize="2xs" variant="subtle" onClick={onCancel}>Cancel</Button>
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
					disabled: entry !== undefined,
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
					selected={suggested === e.id}
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
				<Password bind:password={manualToken} />
			</Label>
		{:else if oauthAppReady && entry}
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis flex gap-2 items-center">
					Scopes
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
				<McpOAuthConnect
					bind:this={oauthConnect}
					server={{ name: entry?.name ?? serverName, url: committedUrl }}
					path={manualPath}
					onDiscovered={(supported) => (discoveryFoundOAuth = supported)}
					workspace={ws}
					onConnected={(path) => onConnected(path)}
				/>
			{/key}
		{/if}

		{#if needsOauthApp && entry}
			<div class="text-2xs text-secondary">
				No {entry.name} OAuth app is configured on this instance. An admin can add one in instance settings
				to enable signing in.
			</div>
		{:else if canDiscover && !$enterpriseLicense}
			<div class="text-2xs text-secondary"
				>Signing in to an MCP server is an enterprise feature.</div
			>
		{/if}

		<Label label="Path">
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
