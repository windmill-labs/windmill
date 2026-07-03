<script lang="ts">
	import { run } from 'svelte/legacy'

	import { userStore, workspaceStore } from '$lib/stores'
	import LabelsInput from './LabelsInput.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import {
		OauthService,
		ResourceService,
		WorkspaceService,
		VariableService,
		type TokenResponse,
		type ResourceType
	} from '$lib/gen'
	import { emptyString, truncateRev, urlize } from '$lib/utils'
	import oauthConnectRegistry from '$oauth_connect_registry'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Path from './Path.svelte'
	import { Button, RadioCard, Skeleton } from './common'
	import ApiConnectForm from './ApiConnectForm.svelte'
	import SearchItems from './SearchItems.svelte'
	import WhitelistIp from './WhitelistIp.svelte'
	import { sendUserToast } from '$lib/toast'
	import OauthScopes from './OauthScopes.svelte'
	import Markdown from 'svelte-exmarkdown'
	import autosize from '$lib/autosize'
	import { base } from '$lib/base'
	import Required from './Required.svelte'
	import Toggle from './Toggle.svelte'
	import { Pen, Search } from 'lucide-svelte'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import { apiTokenApps, forceSecretValue, linkedSecretValue } from './app_connect'
	import type { SchemaProperty } from '$lib/common'
	import TextInput from './text_input/TextInput.svelte'
	import { sameTopDomainOrigin } from '$lib/cookies'
	import SyncResourceTypes from './SyncResourceTypes.svelte'
	import Label from './Label.svelte'

	interface Props {
		step?: number
		resourceType?: string
		isGoogleSignin?: boolean
		disabled?: boolean
		manual?: boolean
		express?: boolean
		workspace?: string
	}

	let {
		step = $bindable(1),
		resourceType = $bindable(''),
		isGoogleSignin = $bindable(false),
		disabled = $bindable(false),
		manual = $bindable(true),
		express = false,
		workspace = undefined
	}: Props = $props()

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)

	let isValid = $state(true)

	const nativeLanguagesCategory = [
		'postgresql',
		'mysql',
		'bigquery',
		'snowflake',
		'mssql',
		'graphql',
		'oracledb'
	]

	let filter = $state('')
	let value: string = $state('')
	let valueToken: TokenResponse | undefined = undefined
	let connects: string[] | undefined = $state(undefined)
	/** Per-provider instance-entry metadata, keyed by provider name. */
	let connectsInfo: Record<
		string,
		{ supports_client_credentials: boolean; has_shared_credentials: boolean }
	> = $state({})

	/** An instance entry with shared credentials (admin id+secret): connect with
	 * no input. Shown under "Instance-configured"; bring-your-own-only providers
	 * (no shared creds) are shown under "Others" instead. */
	function isSharedConnect(key: string): boolean {
		return connectsInfo[key]?.has_shared_credentials ?? false
	}

	const SANDBOX_SUFFIX = '_sandbox'
	function stripSandboxSuffix(name: string): string {
		return name.endsWith(SANDBOX_SUFFIX) ? name.slice(0, -SANDBOX_SUFFIX.length) : name
	}
	// `resourceType` is always the canonical type (e.g. `docusign`) so resource
	// rows are uniform. `connectClient` carries the suffixed OAuth client name
	// (e.g. `docusign_sandbox`) used to look up credentials/URLs at runtime
	// and stored on `account.client` so token refresh hits the right endpoint.
	let connectClient: string = $state('')
	let connectsManual: { key: string; img?: string; instructions: string[] }[] | undefined =
		$state(undefined)
	let args: any = $state({})
	let renderDescription = $state(true)

	function computeCandidates(resourceType: string, argsKeys: string[], passwords: string[]) {
		return apiTokenApps[resourceType]?.linkedSecret
			? ([apiTokenApps[resourceType]?.linkedSecret] as string[])
			: argsKeys.filter(
					(x) =>
						passwords.includes(x) ||
						['token', 'secret', 'key', 'pass', 'private'].some((y) => x.toLowerCase().includes(y))
				)
	}

	let linkedSecrets: string[] = $state([])
	let linkedSecretCandidates: string[] | undefined = $state(undefined)
	function computeDefaultLinkedSecrets(
		resourceType: string,
		argsKeys: string[],
		passwords: string[]
	): string[] {
		linkedSecretCandidates = computeCandidates(resourceType, argsKeys, passwords)
		const forced = forceSecretValue(resourceType)
		if (forced) {
			return [forced]
		}
		const best = linkedSecretCandidates?.sort(
			(ua, ub) => linkedSecretValue(ub) - linkedSecretValue(ua)
		)?.[0]
		return best ? [best] : []
	}

	let scopes: string[] = $state([])
	/** The authorization-code default scopes (instance entry / registry), kept so
	 * toggling back from client-credentials can restore them. */
	let instanceScopes: string[] = $state([])
	let extra_params: [string, string][] = []
	let responseExtra: Record<string, string> = $state({})
	let path: string = $state('')
	let description = $state('')
	let labels: string[] | undefined = $state(undefined)
	let wsSpecific = $state(false)
	let deployTo: string | undefined = $state(undefined)

	/**
	 * Client credentials OAuth flow support
	 * @description Determines if the selected OAuth provider supports client_credentials grant type
	 * alongside the traditional authorization_code flow
	 */
	let supportsClientCredentials = $state(false)

	/**
	 * OAuth flow selection
	 * @description Controls which OAuth flow to use:
	 * - false: authorization_code flow (interactive, requires user consent)
	 * - true: client_credentials flow (server-to-server, no user interaction)
	 */
	let useClientCredentials = $state(false)

	/**
	 * Client credentials for resource-level OAuth
	 */
	let clientId = $state('')
	let clientSecret = $state('')
	let ccInstance = $state('')
	/** Bring-your-own resource-level token endpoint override (optional). Only sent
	 * for non-instance-templated providers, where it isn't host-pinned. */
	let tokenUrl = $state('')

	let resourceTypeInfo: ResourceType | undefined = $state(undefined)
	let resourceTypeNotFound = $state(false)

	function registryEntry(): any {
		const reg = oauthConnectRegistry as Record<string, any>
		// Resolve `_sandbox` clients to their parent registry entry (e.g.
		// salesforce_sandbox -> salesforce) so sandbox connections see CC metadata.
		return reg[stripSandboxSuffix(connectClient)] ?? reg[stripSandboxSuffix(resourceType)]
	}

	/** The static registry declares this provider supports client credentials */
	function registryCcCapable(): boolean {
		return registryEntry()?.grant_types?.includes('client_credentials') ?? false
	}

	/** Instance-name metadata for providers whose token URL is instance-templated
	 * (carried in `connect_config_template`): the user enters an instance name
	 * instead of a full token URL, and the backend substitutes it into the
	 * fixed-host template so the exchange host stays pinned. */
	let ccInstanceMeta = $derived(
		registryEntry()?.connect_config_template as
			| { label: string; placeholder: string; help_url?: string }
			| undefined
	)

	/** Instance entry declares client credentials but not authorization_code
	 * (custom provider configured with only a token URL) */
	let authCodeUnavailable = $state(false)

	/** Instance entry carries shared client-credentials (id + secret); the user
	 * doesn't enter their own — the exchange runs server-side with those creds */
	let ccInstanceConfigured = $state(false)

	/** The user wants their own credentials (picked the provider from the "Others"
	 * section) — overrides the shared instance credentials for this connection */
	let ccBringYourOwn = $state(false)

	/** Connect with the shared instance credentials (no form) rather than the
	 * bring-your-own form */
	let useSharedInstanceCreds = $derived(ccInstanceConfigured && !ccBringYourOwn)

	/** Connectable via client credentials only: registry-declared provider with
	 * no instance OAuth client, or instance provider without an authorize URL */
	let ccOnly = $derived.by(
		() =>
			authCodeUnavailable ||
			(registryCcCapable() && connectClient != '' && !(connects?.includes(connectClient) ?? false))
	)

	/** Clear CC inputs and scopes so a previous selection never leaks into a new one */
	function resetClientCredentialsState() {
		supportsClientCredentials = false
		useClientCredentials = false
		authCodeUnavailable = false
		ccInstanceConfigured = false
		ccBringYourOwn = false
		clientId = ''
		clientSecret = ''
		ccInstance = ''
		tokenUrl = ''
		scopes = []
	}

	/** Default scopes for the client-credentials grant. Registry providers use
	 * their `cc_scopes` (auth-code scopes are invalid in a 2-legged request);
	 * custom (non-registry) providers configured at the instance level have no
	 * registry entry, so they keep their admin-configured scopes (`instanceScopes`)
	 * instead of being zeroed. */
	function defaultCcScopes(): string[] {
		const entry = registryEntry()
		return entry ? (entry.cc_scopes ?? []) : instanceScopes
	}

	function enableClientCredentials() {
		manual = false
		supportsClientCredentials = true
		if (!useClientCredentials) {
			// Switching into client-credentials: default to the CC scopes (never the
			// authorization-code scopes — most providers reject member/consent scopes
			// in a 2-legged request). Only reset on the transition so edits made while
			// already in CC mode are preserved.
			scopes = defaultCcScopes()
		}
		useClientCredentials = true
	}

	/** Switch to the browser sign-in (authorization-code) grant, restoring its
	 * default scopes when coming from the client-credentials grant. */
	function selectAuthCodeGrant() {
		if (useClientCredentials) {
			scopes = instanceScopes
		}
		useClientCredentials = false
	}

	/** Static registry declares client-credentials support for `key`. */
	function isCcCapable(key: string): boolean {
		return (
			(oauthConnectRegistry as Record<string, any>)[stripSandboxSuffix(key)]?.grant_types?.includes(
				'client_credentials'
			) ?? false
		)
	}

	/** Step-1 "Others" selection: CC-capable resource types open the client-
	 * credentials form with the user's own credentials — even when the instance
	 * has shared ones (the "Instance-configured OAuth APIs" section is the entry
	 * point for those). Every other type opens the raw manual form. */
	function selectFromOthers(key: string) {
		connectClient = key
		resourceType = key
		resetClientCredentialsState()
		// Registry CC providers and instance-configured providers that declare the
		// client-credentials grant (incl. custom providers set up with only a token
		// URL and no shared creds) open the bring-your-own form. Everything else is
		// a manual resource.
		if (isCcCapable(key) || (connectsInfo[key]?.supports_client_credentials ?? false)) {
			ccBringYourOwn = true
			enableClientCredentials()
		} else {
			manual = true
		}
		next()
	}

	let pathError = $state('')

	export async function open(rt?: string) {
		if (!rt) {
			loadResourceTypes()
		}
		step = 1 //express && !manual ? 3 : 1
		value = ''
		description = ''
		labels = undefined
		wsSpecific = false
		const rawRt = rt ?? ''
		connectClient = rawRt
		resourceType = stripSandboxSuffix(rawRt)
		valueToken = undefined

		resetClientCredentialsState()

		await loadConnects()
		const inConnects = connects?.includes(connectClient) ?? false
		// Registry-declared client-credentials providers are connectable even
		// without an instance OAuth client
		manual = !inConnects && !(rt && registryCcCapable())
		if (manual && express) {
			dispatch('error', 'Express OAuth setup is not available for non OAuth resource types')
			return
		}
		if (!inConnects && !manual && express) {
			// Client-credentials connections need interactive credential entry
			dispatch('error', 'Express OAuth setup is not available for client credentials providers')
			return
		}
		if (!inConnects && !manual) {
			enableClientCredentials()
		}
		if (rt) {
			if (!manual && express) {
				await getScopesAndParams()
				if (authCodeUnavailable) {
					// No popup flow to drive express setup with
					dispatch('error', 'Express OAuth setup is not available for client credentials providers')
					return
				}
				step = 2
			}
			next()
		}
	}

	async function loadConnects() {
		if (!connects) {
			try {
				const list = (await OauthService.listOauthConnects())
					.filter((x) => x.name != 'supabase_wizard')
					.sort((a, b) => a.name.localeCompare(b.name))
				connects = list.map((x) => x.name)
				connectsInfo = Object.fromEntries(list.map((x) => [x.name, x]))
			} catch (e) {
				connects = []
				connectsInfo = {}
				console.error('Error loading OAuth connects', e)
			}
		}
	}

	run(() => {
		isGoogleSignin =
			step == 1 &&
			(resourceType == 'google' ||
				resourceType == 'gmail' ||
				resourceType == 'gcal' ||
				resourceType == 'gdrive' ||
				resourceType == 'gsheets')
	})

	run(() => {
		disabled =
			(step == 1 && resourceType == '') ||
			(step == 2 &&
				(manual
					? value == '' &&
						args &&
						args['token'] == '' &&
						args['password'] == '' &&
						args['api_key'] == '' &&
						args['key'] == '' &&
						linkedSecrets.length > 0
					: useClientCredentials &&
						!useSharedInstanceCreds &&
						(clientId.trim() == '' ||
							clientSecret.trim() == '' ||
							(!!ccInstanceMeta && ccInstance.trim() == '')))) ||
			step == 3 ||
			(step == 4 && pathError != '') ||
			!isValid
	})

	export async function loadResourceTypes() {
		if (connectsManual) {
			return
		}
		const availableRts = await ResourceService.listResourceTypeNames({
			workspace: effectiveWorkspace
		})

		// "Others" lists every resource type — including instance-configured OAuth
		// providers — so any of them can also be connected with the user's own
		// credentials or manually, not only via the shared instance setup (same as
		// the authorization-code behavior).
		connectsManual = availableRts
			.map(
				(x) =>
					({
						key: x,
						...(apiTokenApps[x] ?? {
							instructions: '',
							img: undefined,
							linkedSecret: undefined
						})
					}) as { key: string; img?: string; instructions: string[] }
			)
			.sort((a, b) => a.key.localeCompare(b.key))
		const filteredNativeLanguages = filteredConnectsManual?.filter(
			(o) => nativeLanguagesCategory?.includes(o[0]) ?? false
		)

		try {
			filteredConnectsManual = [
				...(filteredNativeLanguages ?? []),
				...(filteredConnectsManual ?? []).filter(
					({ key }) => !nativeLanguagesCategory.includes(key)
				)
			]
		} catch (e) {}
	}

	function popupListener(event) {
		console.log('Received oauth popup message', event)
		let data = event.data
		if (!sameTopDomainOrigin(event.origin, window.location.origin)) {
			console.log(
				'Received oauth popup message from different origin',
				event.origin,
				window.location.origin
			)
			return
		}

		if (data.type == 'success' || data.type == 'error') {
			window.removeEventListener('message', popupListener)
			processPopupData(data)
		}
	}

	function handleStorageEvent(event) {
		if (event.key === 'oauth-callback') {
			try {
				processPopupData(JSON.parse(event.newValue))
				console.log('OAuth from storage', event.newValue)
				// Clean up
				localStorage.removeItem('oauth-callback')
				window.removeEventListener('storage', handleStorageEvent)
			} catch (e) {
				console.error('Error processing oauth-callback', e)
			}
		} else {
			console.log('Storage event', event.key)
		}
	}

	onDestroy(() => {
		window.removeEventListener('message', popupListener)
		window.removeEventListener('storage', handleStorageEvent)
	})

	$effect(() => {
		if (!effectiveWorkspace) {
			deployTo = undefined
			return
		}

		WorkspaceService.getDeployTo({ workspace: effectiveWorkspace }).then((x) => {
			deployTo = x.deploy_to
		})
	})

	function processPopupData(data) {
		console.log('Processing oauth popup data')
		if (data.type === 'error') {
			sendUserToast(data.error, true)
			step = 2
		} else if (data.type === 'success') {
			connectClient = data.resource_type
			resourceType = stripSandboxSuffix(connectClient)
			value = data.res.access_token!
			valueToken = data.res
			responseExtra = data.extra ?? {}
			step = 4
			if (express) {
				path = `u/${$userStore?.username}/${resourceType}_${new Date().getTime()}`
				next()
			}
		}
	}

	async function getScopesAndParams() {
		if (!connects?.includes(connectClient)) {
			// No instance OAuth client (registry-declared CC-only provider):
			// defaults come from the static registry instead.
			instanceScopes = registryEntry()?.scopes ?? []
			scopes = useClientCredentials ? defaultCcScopes() : instanceScopes
			extra_params = []
			supportsClientCredentials = registryCcCapable()
			return
		}
		const connect = await OauthService.getOauthConnect({ client: connectClient })
		instanceScopes = connect.scopes ?? []
		extra_params = Object.entries(connect.extra_params ?? {}) as [string, string][]

		/**
		 * The CC flow is offered when the static registry declares it for the
		 * provider, or the admin enabled it on the instance entry (custom
		 * providers)
		 */
		supportsClientCredentials =
			registryCcCapable() || (connect.grant_types?.includes('client_credentials') ?? false)
		// Shared instance credentials: the user connects without entering any creds
		ccInstanceConfigured = connect.client_credentials_configured ?? false
		// Custom provider configured with only a token URL: no popup flow possible
		authCodeUnavailable =
			supportsClientCredentials && !(connect.grant_types?.includes('authorization_code') ?? true)
		if (authCodeUnavailable) {
			useClientCredentials = true
		}
		// Default scopes to the active grant: client-credentials uses the registry's
		// cc_scopes (auth-code scopes are invalid in a 2-legged request), every other
		// path keeps the instance entry's scopes. Applies to shared instance creds,
		// not just bring-your-own. Switching grants resets to these defaults.
		scopes = useClientCredentials ? defaultCcScopes() : instanceScopes
	}

	async function getResourceTypeInfo() {
		try {
			resourceTypeNotFound = false
			resourceTypeInfo = await ResourceService.getResourceType({
				workspace: effectiveWorkspace,
				path: resourceType
			})
			const props: Record<string, SchemaProperty> = resourceTypeInfo?.schema?.['properties'] ?? {}
			const newArgsKeys = Object.keys(props).filter((x) => props?.[x]?.type == 'string') ?? []

			const passwords = newArgsKeys.filter((x) => {
				return props?.[x]?.password
			})
			if (linkedSecrets.length === 0) {
				linkedSecrets = computeDefaultLinkedSecrets(resourceType, newArgsKeys, passwords)
			}
		} catch (err) {
			resourceTypeInfo = undefined
			resourceTypeNotFound = true
		}
	}
	export async function next() {
		if (step == 1) {
			linkedSecrets = []
			if (manual) {
				getResourceTypeInfo()
				args = {}
			} else {
				getResourceTypeInfo()
				getScopesAndParams()
			}
			step += 1
		} else if (step == 2 && !manual) {
			if (useClientCredentials) {
				/**
				 * Client credentials flow: Direct API call to backend
				 * No popup window or user interaction required — the resource-level
				 * credentials are exchanged directly against the token URL
				 */
				try {
					// Trim whitespace from credentials to avoid false negatives
					const trimmedClientId = clientId.trim()
					const trimmedClientSecret = clientSecret.trim()
					const trimmedInstance = ccInstance.trim()
					// Instance-templated providers collect an instance name; the backend
					// builds the host-pinned token URL from it. Other registry providers
					// need no URL input (the token URL comes from the registry).
					const needsInstance = !!ccInstanceMeta

					// Bring-your-own credentials are required unless the provider has
					// shared instance credentials, in which case the exchange runs
					// server-side with those and no input is collected here.
					if (
						!useSharedInstanceCreds &&
						(!trimmedClientId || !trimmedClientSecret || (needsInstance && !trimmedInstance))
					) {
						sendUserToast(
							needsInstance
								? `Client ID, Client Secret and ${ccInstanceMeta?.label} are required for client credentials flow`
								: 'Client ID and Client Secret are required for client credentials flow',
							true
						)
						return
					}

					const tokenResponse = await OauthService.connectClientCredentials({
						workspace: effectiveWorkspace,
						client: connectClient,
						requestBody: useSharedInstanceCreds
							? { scopes: scopes }
							: {
									scopes: scopes,
									cc_client_id: trimmedClientId,
									cc_client_secret: trimmedClientSecret,
									// Instance-templated providers are host-pinned via the instance
									// name; only other providers accept a free-form token URL override.
									...(needsInstance
										? { cc_instance: trimmedInstance }
										: tokenUrl.trim()
											? { cc_token_url: tokenUrl.trim() }
											: {})
								}
					})

					// Process the token response like in popup flow
					value = tokenResponse.access_token!
					valueToken = {
						...tokenResponse,
						grant_type: 'client_credentials' // Mark this token as client_credentials
					}
					step = 4
					if (express) {
						path = `u/${$userStore?.username}/${resourceType}_${new Date().getTime()}`
						next()
					}
				} catch (error) {
					sendUserToast(
						`Failed to connect with client credentials: ${error.body || error.message}`,
						true
					)
				}
			} else {
				/**
				 * Authorization code flow: Traditional OAuth popup window
				 * Requires user interaction and consent
				 * Opens popup for user to authenticate with OAuth provider
				 */
				const url = new URL(`/api/oauth/connect/${connectClient}`, window.location.origin)
				url.searchParams.append('scopes', scopes.join('+'))
				if (extra_params.length > 0) {
					extra_params.forEach(([key, value]) => url.searchParams.append(key, value))
				}
				window.addEventListener('message', popupListener)
				window.addEventListener('storage', handleStorageEvent)
				console.log('opening popup', url.toString())
				window.open(url.toString(), '_blank', 'popup=true')
				step += 1
			}
		} else {
			if (!path) {
				if (step == 2) return
				throw Error('Path is not set')
			}
			// Check if variable paths already exist
			if (!manual || linkedSecrets.length <= 1) {
				const exists = await VariableService.existsVariable({
					workspace: effectiveWorkspace,
					path
				})
				if (exists) {
					throw Error(`Variable at path ${path} already exists. Delete it or pick another path`)
				}
			} else {
				for (const secretField of linkedSecrets) {
					const varPath = `${path}_${secretField}`
					const exists = await VariableService.existsVariable({
						workspace: effectiveWorkspace,
						path: varPath
					})
					if (exists) {
						throw Error(
							`Variable at path ${varPath} already exists. Delete it or pick another path`
						)
					}
				}
			}
			let exists = await ResourceService.existsResource({
				workspace: effectiveWorkspace,
				path
			})

			if (exists) {
				throw Error(`Resource at path ${path} already exists. Delete it or pick another path`)
			}

			// Per-instance OAuth providers (Snowflake, ServiceNow, …): fill the
			// resource args from the connection's instance, per the registry
			// template's resource_mapping (e.g. ServiceNow -> instance_url:
			// https://{instance}.service-now.com). Bring-your-own carries the instance
			// the user entered in `ccInstance` (raw, possibly a full host); the shared
			// path carries it (already normalized) in the connect entry's extra_params.
			// Prefer the user-entered one so the saved resource matches the exchange.
			const connectTemplate = (oauthConnectRegistry as Record<string, any>)[resourceType]
				?.connect_config_template
			if (connectTemplate?.resource_mapping) {
				const instanceKey = connectTemplate.extra_params_key ?? 'instance'
				let instanceValue = extra_params.find(([key, _]) => key === instanceKey)?.[1] ?? ''
				if (ccInstance.trim()) {
					const stripSuffix = connectTemplate.strip_suffix as string | undefined
					let v = ccInstance
						.trim()
						.replace(/^https?:\/\//, '')
						.replace(/\/.*$/, '')
					if (stripSuffix && v.endsWith(stripSuffix)) {
						v = v.slice(0, -stripSuffix.length)
					}
					instanceValue = v.replace(/\.+$/, '')
				}
				if (instanceValue) {
					for (const [argField, valueTemplate] of Object.entries(
						connectTemplate.resource_mapping as Record<string, string>
					)) {
						args[argField] = valueTemplate.replaceAll('{instance}', instanceValue)
					}
				}
			}
			if (resourceType === 'quickbooks' && responseExtra['realmId']) {
				args['realmId'] = responseExtra['realmId']
			}

			let account: number | undefined = undefined
			if (valueToken?.expires_in != undefined) {
				const accountData: any = {
					refresh_token: valueToken.refresh_token ?? '',
					expires_in: valueToken.expires_in,
					client: connectClient,
					grant_type: valueToken.grant_type || 'authorization_code'
				}

				// Store scopes so token refresh uses the same scopes
				if (scopes.length > 0) {
					accountData.scopes = scopes
				}

				// Client-credentials accounts are self-contained: the refresh worker
				// re-exchanges using only what is stored on the account row. With
				// shared instance credentials the backend copies them onto the row,
				// so nothing is sent from here.
				if (useClientCredentials && !useSharedInstanceCreds) {
					accountData.cc_client_id = clientId.trim()
					accountData.cc_client_secret = clientSecret.trim()
					// Instance-templated providers send an instance name; the backend
					// resolves and stores the host-pinned token URL. Other providers may
					// send an optional token URL override (stored for refresh); without
					// it the token URL comes from the registry/instance config.
					if (ccInstanceMeta) {
						accountData.cc_instance = ccInstance.trim()
					} else if (tokenUrl.trim()) {
						accountData.cc_token_url = tokenUrl.trim()
					}
				}

				account = Number(
					await OauthService.createAccount({
						workspace: effectiveWorkspace,
						requestBody: accountData
					})
				)
			}

			const resourceValue = args

			let savedVariableCount = 0
			if (!manual) {
				// OAuth flow: single secret variable for the token
				if (typeof value == 'string' && value != '' && !value.startsWith('$var:')) {
					savedVariableCount++
					await VariableService.createVariable({
						workspace: effectiveWorkspace,
						requestBody: {
							path,
							value: value,
							is_secret: true,
							description: emptyString(description)
								? `OAuth token for ${resourceType}`
								: description,
							is_oauth: true,
							account: account,
							ws_specific: wsSpecific
						}
					})
					resourceValue['token'] = `$var:${path}`
				}
			} else if (linkedSecrets.length === 1) {
				// Single secret: use the resource path as variable name (original behavior)
				const secretField = linkedSecrets[0]
				const v = args[secretField]
				if (typeof v == 'string' && v != '' && !v.startsWith('$var:')) {
					savedVariableCount++
					await VariableService.createVariable({
						workspace: effectiveWorkspace,
						requestBody: {
							path,
							value: v,
							is_secret: true,
							description: emptyString(description) ? `Token for ${resourceType}` : description,
							is_oauth: false,
							ws_specific: wsSpecific
						}
					})
					resourceValue[secretField] = `$var:${path}`
				}
			} else if (linkedSecrets.length > 1) {
				// Multiple secrets: append _field_name to each variable path
				for (const secretField of linkedSecrets) {
					const v = args[secretField]
					if (typeof v == 'string' && v != '' && !v.startsWith('$var:')) {
						const varPath = `${path}_${secretField}`
						savedVariableCount++
						await VariableService.createVariable({
							workspace: effectiveWorkspace,
							requestBody: {
								path: varPath,
								value: v,
								is_secret: true,
								description: emptyString(description)
									? `${secretField} for ${resourceType}`
									: description,
								is_oauth: false,
								ws_specific: wsSpecific
							}
						})
						resourceValue[secretField] = `$var:${varPath}`
					}
				}
			}

			await ResourceService.createResource({
				workspace: effectiveWorkspace,
				requestBody: {
					resource_type: resourceType,
					path,
					value: resourceValue,
					description,
					labels,
					ws_specific: wsSpecific
				}
			})
			dispatch('refresh', path)
			dispatch('close')
			sendUserToast(
				`Saved resource${savedVariableCount > 0 ? ` and ${savedVariableCount} variable${savedVariableCount > 1 ? 's' : ''}` : ''} path: ${path}`
			)
			step = 1
			resourceType = ''
			connectClient = ''
		}
	}

	export async function back() {
		if (step == 4) {
			step -= 2
		} else if (step > 1) {
			step -= 1
		}
		if (step == 1) {
			loadConnects()
			loadResourceTypes()
		}
	}

	const dispatch = createEventDispatcher<{ error: string; refresh: string; close: void }>()

	let filteredConnects: { key: string }[] = $state([])
	let filteredConnectsManual: { key: string; img?: string; instructions: string[] }[] = $state([])

	let editScopes = $state(false)
</script>

{#if !express}
	<SearchItems
		{filter}
		items={connects
			? connects.filter(isSharedConnect).map((key) => ({
					key
				}))
			: undefined}
		bind:filteredItems={filteredConnects}
		f={(x) => x.key}
	/>
	<SearchItems
		{filter}
		items={connectsManual}
		bind:filteredItems={filteredConnectsManual}
		f={(x) => x.key}
	/>
	{#if step == 1}
		<div class="pb-2 my-1">
			<div class="relative w-full">
				<Search class="absolute left-2 top-1/2 -translate-y-1/2 text-tertiary" size={14} />
				<TextInput
					inputProps={{ placeholder: 'Search resource type', id: 'search-resource-type' }}
					bind:value={filter}
					class="pl-7 text-xs w-full"
				/>
			</div>
		</div>

		<h2 class="mb-4 text-sm font-semibold text-emphasis">Instance-configured OAuth APIs</h2>
		<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center">
			{#if filteredConnects}
				{#each filteredConnects as { key }}
					<Button
						unifiedSize="md"
						variant="default"
						selected={key === connectClient}
						on:click={() => {
							manual = false
							connectClient = key
							resourceType = stripSandboxSuffix(key)
							resetClientCredentialsState()
							next()
						}}
					>
						<IconedResourceType name={key} after={true} width="20px" height="20px" />
					</Button>
				{/each}
			{:else}
				{#each new Array(3) as _}
					<Skeleton layout={[[2]]} />
				{/each}
			{/if}
		</div>
		{#if connects && connects.filter(isSharedConnect).length == 0}
			<div class="text-secondary text-xs w-full"
				>No OAuth APIs have been set up on this instance. To add OAuth APIs, first sync the resource
				types with the hub, then add OAuth configuration. See <a
					href="https://www.windmill.dev/docs/misc/setup_oauth">documentation</a
				>
			</div>
		{/if}

		<h2 class="mt-8 mb-2 text-sm font-semibold text-emphasis">Others</h2>

		{#if connectsManual && connectsManual?.length < 10}
			<div class="text-secondary text-xs p-2">
				Resource types have not been synced with the hub
			</div>
		{/if}

		<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
			{#if filteredConnectsManual}
				{#each filteredConnectsManual as { key }}
					{#if nativeLanguagesCategory.includes(key)}
						<Button
							unifiedSize="md"
							variant="default"
							selected={key === resourceType}
							on:click={() => selectFromOthers(key)}
						>
							<IconedResourceType name={key} after={true} width="20px" height="20px" />
						</Button>
					{/if}
				{/each}
			{/if}
			{#if filteredConnectsManual}
				{#each filteredConnectsManual as { key }}
					{#if !nativeLanguagesCategory.includes(key)}
						<!-- Exclude specific items -->
						<Button
							aiId={`app-connect-inner-${key}`}
							aiDescription={`Connect to ${key}`}
							unifiedSize="md"
							variant="default"
							selected={key === resourceType}
							on:click={() => selectFromOthers(key)}
						>
							<IconedResourceType name={key} after={true} width="20px" height="20px" />
						</Button>
					{/if}
				{/each}
			{:else}
				{#each new Array(9) as _}
					<Skeleton layout={[[2]]} />
				{/each}
			{/if}
		</div>
		<div class="mt-6">
			<SyncResourceTypes
				onSynced={async () => {
					connectsManual = undefined
					await loadResourceTypes()
					connects = undefined
					await loadConnects()
				}}
			/>
		</div>
	{:else if step == 2 && manual}
		<div class="flex flex-col gap-8">
			<Label label="Path">
				<Path
					bind:error={pathError}
					bind:path
					initialPath=""
					namePlaceholder={resourceType}
					kind="resource"
				/>
			</Label>
			<LabelsInput bind:labels class="-mt-5" />
			{#if deployTo}
				<Label
					label="Workspace specific"
					tooltip="Prevents this resource from being deployed to prod/staging"
				>
					<Toggle bind:checked={wsSpecific} />
				</Label>
			{/if}

			{#if apiTokenApps[resourceType]}
				<h2 class="mt-4 mb-2">Instructions</h2>
				<div class="pl-10">
					<ol class="list-decimal">
						{#each apiTokenApps[resourceType].instructions as step}
							<li>
								{@html step}
							</li>
						{/each}
					</ol>
				</div>
				{#if apiTokenApps[resourceType].img}
					<div class="mt-4 w-full overflow-hidden">
						<img
							class="m-auto max-h-60"
							alt="connect"
							src={base + apiTokenApps[resourceType].img}
						/>
					</div>
				{/if}
			{:else if !emptyString(resourceTypeInfo?.description)}
				<label class="flex flex-col gap-1">
					<span class="text-sm font-semibold text-emphasis">
						{resourceTypeInfo?.name} description
					</span>
					<div class="text-xs text-primary font-normal">
						<Markdown md={urlize(resourceTypeInfo?.description ?? '', 'md')} />
					</div>
				</label>
			{/if}
			{#if resourceType == 'postgresql' || resourceType == 'mysql' || resourceType == 'mongodb'}
				<WhitelistIp />
			{/if}

			<div class="flex flex-col gap-1">
				<label class="inline-flex items-center gap-2" for="resource-description">
					<span class="text-xs font-semibold text-emphasis">Resource description</span>
					<Required required={false} />
					<div class="flex gap-1 items-center">
						<Toggle size="xs" bind:checked={renderDescription} />
						<Pen size={14} />
					</div>
				</label>
				{#if renderDescription}
					<div>
						<div class="flex flex-row-reverse text-2xs text-primary -mt-4">GH Markdown</div>
						<textarea
							id="resource-description"
							use:autosize
							bind:value={description}
							placeholder={'Resource description'}
						></textarea>
					</div>
				{:else if description == undefined || description == ''}
					<div class="text-xs text-primary font-normal">No description provided</div>
				{:else}
					<GfmMarkdown md={description} />
				{/if}
			</div>

			{#if resourceTypeNotFound}
				<div class="flex flex-col gap-2 mb-4">
					<p class="text-red-500 dark:text-red-400 text-xs">
						Resource type '{resourceType}' not found in your workspace
					</p>
					<SyncResourceTypes onSynced={getResourceTypeInfo} />
				</div>
			{/if}
			{#if registryCcCapable()}
				<button
					onclick={() => enableClientCredentials()}
					class="text-xs font-normal text-accent w-fit -mt-4"
				>
					Acquire the token automatically via client credentials instead
				</button>
			{/if}
			{#key resourceTypeInfo}
				<ApiConnectForm
					bind:linkedSecrets
					bind:description
					{linkedSecretCandidates}
					{resourceType}
					{resourceTypeInfo}
					bind:args
					bind:isValid
					onSynced={getResourceTypeInfo}
				/>
			{/key}
		</div>
	{:else if step == 2 && !manual}
		{#if manual == false && resourceType != ''}
			<div class="flex flex-col gap-8">
				<div class="flex flex-col gap-1">
					<h2 class="text-lg font-semibold text-emphasis">{resourceType}</h2>
					<div class="text-primary font-normal text-xs"
						>Create a resource backed by an OAuth connection, whose token is fetched from the
						external services and refreshed automatically if needed before expiration.</div
					>
					{#if ccBringYourOwn}
						<button
							onclick={() => {
								manual = true
								useClientCredentials = false
							}}
							class="text-xs font-normal text-accent w-fit mt-2"
						>
							Create resource manually instead
						</button>
					{/if}
				</div>

				{#if resourceTypeInfo?.description}
					<div class="flex flex-col gap-1">
						<h3 class="text-sm font-semibold text-emphasis">Description</h3>
						<div class="text-xs text-primary font-normal">
							<Markdown md={urlize(resourceTypeInfo?.description ?? '', 'md')} />
						</div>
					</div>
				{/if}

				<LabelsInput bind:labels class="-mt-5" />

				{#if supportsClientCredentials}
					<div class="flex flex-col gap-1">
						<h3 class="text-sm font-semibold text-emphasis mb-1">Authentication</h3>
						{#if ccOnly || ccBringYourOwn}
							<div class="text-xs text-secondary font-normal mb-2">
								{#if useSharedInstanceCreds}
									{resourceType} connects server-to-server using the credentials configured for this
									instance. The token is acquired and refreshed automatically.
								{:else}
									{resourceType} connects server-to-server. Enter a client ID and secret; the token is
									acquired and refreshed automatically.
								{/if}
							</div>
						{:else}
							<div class="flex flex-col gap-2 mb-2">
								<RadioCard
									label={`Sign in through ${resourceType}`}
									description="Opens a browser window to log in and authorize. Connects as you."
									selected={!useClientCredentials}
									onSelect={selectAuthCodeGrant}
								/>
								<RadioCard
									label={useSharedInstanceCreds
										? 'Use the configured instance credentials'
										: 'Use a client ID and secret'}
									description={useSharedInstanceCreds
										? "Runs server-to-server with this instance's credentials. No input needed."
										: 'Runs server-to-server. Best for automation or service accounts.'}
									selected={useClientCredentials}
									onSelect={() => enableClientCredentials()}
								/>
							</div>
						{/if}

						{#if useClientCredentials && !useSharedInstanceCreds}
							<form class="flex flex-col gap-6">
								<label class="flex flex-col gap-1">
									<span class="text-xs font-semibold text-emphasis">Client ID</span>
									<TextInput
										bind:value={clientId}
										inputProps={{ placeholder: 'Enter OAuth client ID', required: true }}
									/>
								</label>
								<label class="flex flex-col gap-1">
									<span class="text-xs font-semibold text-emphasis">Client secret</span>
									<TextInput
										inputProps={{
											type: 'password',
											placeholder: 'Enter OAuth client secret',
											required: true
										}}
										bind:value={clientSecret}
									/>
								</label>
								{#if ccInstanceMeta}
									<label class="flex flex-col gap-1">
										<span class="text-xs font-semibold text-emphasis">{ccInstanceMeta.label}</span>
										<div class="text-xs text-secondary font-normal">
											Used to build this provider's token endpoint, stored with the connection for
											automatic token refresh
										</div>
										<TextInput
											inputProps={{ placeholder: ccInstanceMeta.placeholder, required: true }}
											bind:value={ccInstance}
										/>
									</label>
								{:else}
									<label class="flex flex-col gap-1">
										<span class="text-xs font-semibold text-emphasis"
											>Token URL override (optional)</span
										>
										<div class="text-xs text-secondary font-normal">
											Override the provider's token endpoint for this resource, stored with the
											connection and reused on token refresh
										</div>
										<TextInput
											inputProps={{
												type: 'url',
												placeholder: 'https://provider.example.com/oauth/token'
											}}
											bind:value={tokenUrl}
										/>
									</label>
								{/if}
							</form>
						{/if}
					</div>
				{/if}

				<div class="flex flex-col gap-1">
					<h3 class="text-xs font-semibold text-emphasis flex gap-4"
						>Scopes <button
							onclick={() => {
								editScopes = !editScopes
							}}><Pen size={14} /></button
						></h3
					>

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
			</div>
		{/if}
	{:else if step == 3 && !manual && !express}
		{#if useClientCredentials}
			<span class="text-xs text-primary font-normal"> Connecting with client credentials... </span>
		{:else}
			<span class="text-xs text-primary font-normal"> Finish connection in popup window </span>
		{/if}
	{:else}
		<Label label="Path">
			<Path
				initialPath=""
				namePlaceholder={resourceType}
				bind:error={pathError}
				bind:path
				kind="resource"
			/>
		</Label>
		<LabelsInput bind:labels class="-mt-5" />
		{#if deployTo}
			<Label
				label="Workspace specific"
				tooltip="Prevents this resource from being deployed to prod/staging"
			>
				<Toggle bind:checked={wsSpecific} />
			</Label>
		{/if}
		{#if apiTokenApps[resourceType] || !manual}
			<ul class="mt-6">
				<li class="text-xs text-primary font-normal">
					1. A secret variable containing the {apiTokenApps[resourceType]?.linkedSecret ?? 'token'}
					<span class="font-semibold text-emphasis">{truncateRev(value, 5, '*****')}</span>
					will be stored a
					<span class="font-mono whitespace-nowrap text-emphasis">{path}</span>.
				</li>
				<li class="mt-2 text-xs text-primary font-normal">
					2. The resource containing that token will be stored at the same path <span
						class="font-mono whitespace-nowrap text-emphasis">{path}</span
					>. The Variable and Resource will be "linked together", they will be deleted and renamed
					together.
				</li></ul
			>
		{/if}
	{/if}
{/if}
