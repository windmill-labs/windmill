<script lang="ts">
	import { run } from 'svelte/legacy'

	import { userStore, workspaceStore } from '$lib/stores'
	import LabelsInput from './LabelsInput.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import {
		isCustomResourceTypeName,
		resourceTypeDisplayName,
		resourceTypeMatchRank,
		resourceTypeSearchText,
		sortResourceTypesByMatch
	} from './resourceTypeDisplay'
	import {
		OauthService,
		ResourceService,
		WorkspaceService,
		VariableService,
		type TokenResponse,
		type ResourceType
	} from '$lib/gen'
	import { emptyString, truncateRev, urlize } from '$lib/utils'
	import { registryEntryFor, registryCcCapableFor, stripSandboxSuffix } from './oauthRegistry'
	import { createEventDispatcher, onDestroy, tick } from 'svelte'
	import Path from './Path.svelte'
	import { ListRow, RadioCard, Skeleton } from './common'
	import { useListHighlight } from './common/listRow/listHighlight.svelte'
	import ApiConnectForm from './ApiConnectForm.svelte'
	import SearchItems from './SearchItems.svelte'
	import WhitelistIp from './WhitelistIp.svelte'
	import { sendUserToast } from '$lib/toast'
	import OauthScopes from './OauthScopes.svelte'
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
	import ResourcePathHint from './ResourcePathHint.svelte'

	interface Props {
		step?: number
		resourceType?: string
		isGoogleSignin?: boolean
		disabled?: boolean
		manual?: boolean
		express?: boolean
		workspace?: string
		/**
		 * Fill an existing resource instead of creating one. The path is fixed to it and the
		 * "already exists" guard becomes an update, so a caller holding a resource that is
		 * already there — the import wizard's empty stubs — can connect into it rather than
		 * making the user delete it first and retype the path.
		 *
		 * Opt-in: without it this flow still refuses to write over anything.
		 */
		fillPath?: string
	}

	let {
		step = $bindable(1),
		resourceType = $bindable(''),
		isGoogleSignin = $bindable(false),
		disabled = $bindable(false),
		manual = $bindable(true),
		express = false,
		workspace = undefined,
		fillPath = undefined
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

	const SEARCH_INPUT_ID = 'search-resource-type'
	let searchInput: { focus: () => void } | undefined = $state(undefined)

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

	// `resourceType` is always the canonical type (e.g. `docusign`) so resource
	// rows are uniform. `connectClient` carries the suffixed OAuth client name
	// (e.g. `docusign_sandbox`) used to look up credentials/URLs at runtime
	// and stored on `account.client` so token refresh hits the right endpoint.
	let connectClient: string = $state('')
	let connectsManual: { key: string; img?: string; instructions: string[] }[] | undefined =
		$state(undefined)
	let resourceTypeDescriptions: Record<string, string> = $state({})
	// Types made in this workspace, by the `c_` prefix the resources page adds or by the
	// workspace they live in — the hub sync writes its own into `admins`, which every
	// workspace reads from. `created_by` looks like the same signal but isn't: seeded hub
	// types carry a username too.
	let customResourceTypes: Set<string> = $state(new Set())

	// Hub descriptions are markdown; a row shows one line of it, where fenced blocks and
	// backticks read as noise.
	const plainDescription = (d: string) =>
		d
			.replace(/```[\s\S]*?```/g, '')
			.replace(/`/g, '')
			.replace(/\s+/g, ' ')
			.trim()
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

	// Both resolve `_sandbox` clients to their parent entry (e.g. salesforce_sandbox ->
	// salesforce) so sandbox connections see the same metadata. Shared with callers that
	// decide whether to open this dialog at all, so the two cannot disagree.
	function registryEntry(): any {
		return registryEntryFor(connectClient, resourceType)
	}

	/** The static registry declares this provider supports client credentials */
	function registryCcCapable(): boolean {
		return registryCcCapableFor(connectClient, resourceType)
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
		return registryCcCapableFor(key)
	}

	/** Step-1 "Others" selection: CC-capable resource types open the client-
	 * credentials form with the user's own credentials — even when the instance
	 * has shared ones (the "Instance-configured OAuth APIs" section is the entry
	 * point for those). Every other type opens the raw manual form. */
	function connectOauth(key: string) {
		manual = false
		connectClient = key
		resourceType = stripSandboxSuffix(key)
		resetClientCredentialsState()
		next()
	}

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
		// The list is keyboard-driven from the search field, so it takes focus on open.
		tick().then(() => searchInput?.focus())
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

	// Google's terms require its own button on the control that starts the sign-in, which is
	// the step-2 Connect: step 1 only picks a type, and a manual step 2 saves a resource
	// without ever reaching Google.
	run(() => {
		isGoogleSignin =
			step == 2 &&
			!manual &&
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
		// The prefix alone identifies a workspace-made type, and it rides on the names call the
		// list already needs — so the custom section survives the full list below 403ing.
		customResourceTypes = new Set(availableRts.filter(isCustomResourceTypeName))

		// Descriptions only feed search, so they are fetched off the critical path and
		// allowed to fail: `resources/type/list` is not on the public app domain's route
		// allow-list (`listnames` is), and it carries every type's full schema. Awaiting it
		// would hold the list behind a request nothing on screen needs -- in a published
		// app, behind one that is guaranteed to 403. resourceTypeDescriptions feeds a
		// $derived, so search re-ranks when they land.
		ResourceService.listResourceType({ workspace: effectiveWorkspace })
			.then((types) => {
				resourceTypeDescriptions = Object.fromEntries(
					types.filter((t) => t.description).map((t) => [t.name, t.description!])
				)
				// A type sitting in this workspace was made here too, but only the full list carries
				// `workspace_id`. Inside `admins` the two are indistinguishable — every type lives
				// there — so the prefix is all there is to go on.
				customResourceTypes = new Set([
					...customResourceTypes,
					...types.filter((t) => t.workspace_id && t.workspace_id !== 'admins').map((t) => t.name)
				])
			})
			.catch(() => {})

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
			// `fillPath` decides the path as surely as express does, so neither stops here.
			if (fillPath || express) {
				path = fillPath ?? `u/${$userStore?.username}/${resourceType}_${new Date().getTime()}`
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
				// Awaited: the popup is built from `scopes`, so advancing before this
				// resolves sends the user to an authorize url with no scope at all.
				await getScopesAndParams()
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
					if (fillPath || express) {
						path = fillPath ?? `u/${$userStore?.username}/${resourceType}_${new Date().getTime()}`
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

			// Filling one names its path up front; anything else reaching an occupied path got
			// there by the user typing it, which is the case worth refusing.
			//
			// The type is checked here and not only by the caller: `fillPath` says "write into
			// this path", and a path says nothing about what lives at it. A workspace resource
			// of another type sitting where the project wanted one of ours would otherwise have
			// its value replaced with credentials for a different provider, while keeping its
			// own type — destroying a working resource that has nothing to do with the import.
			const filling = exists && !!fillPath && path === fillPath
			if (filling) {
				// Fails closed. Only a read that succeeds and answers with exactly this type
				// permits the write — a failed read, a missing type, or any other type all
				// refuse. Letting "could not tell" through is how the overwrite this guard
				// exists to stop would happen anyway, on the one occasion the check was needed
				// and could not run.
				let occupantType: string | undefined
				try {
					occupantType = (
						await ResourceService.getResource({ workspace: effectiveWorkspace, path })
					)?.resource_type
				} catch (e: any) {
					throw Error(
						`Could not read what is already at ${path} (${e?.body ?? e?.message ?? e}), ` +
							`so it will not be written over. Try again.`
					)
				}
				if (occupantType !== resourceType) {
					throw Error(
						`Resource at path ${path} is ${
							occupantType ? `a ${occupantType} resource` : 'of an unknown type'
						}, not ${resourceType}. Move or rename it, then import again.`
					)
				}
			}
			if (exists && !filling) {
				throw Error(`Resource at path ${path} already exists. Delete it or pick another path`)
			}

			// Per-instance OAuth providers (Snowflake, ServiceNow, …): fill the
			// resource args from the connection's instance, per the registry
			// template's resource_mapping (e.g. ServiceNow -> instance_url:
			// https://{instance}.service-now.com). Bring-your-own carries the instance
			// the user entered in `ccInstance` (raw, possibly a full host); the shared
			// path carries it (already normalized) in the connect entry's extra_params.
			// Prefer the user-entered one so the saved resource matches the exchange.
			const connectTemplate = registryEntryFor(resourceType)?.connect_config_template
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

			if (filling) {
				// The stub the import made carries no description, so this is the one chance to
				// give it one; its resource_type and path are already what we want.
				await ResourceService.updateResource({
					workspace: effectiveWorkspace,
					path,
					requestBody: { value: resourceValue, description }
				})
			} else {
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
			}
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

	// uFuzzy scores the name and the description as one string, so searching "google" ranks
	// every type whose description mentions Google alongside the ones named after it. Re-sort
	// on which field matched, keeping uFuzzy's order within a tier.
	const rank = (items: { key: string }[] | undefined) =>
		items &&
		sortResourceTypesByMatch(
			items,
			filter,
			(x) => x.key,
			(x) => resourceTypeDescriptions[x.key]
		)
	let rankedConnects = $derived(rank(filteredConnects))
	let rankedConnectsManual = $derived(
		rank(filteredConnectsManual) as typeof filteredConnectsManual | undefined
	)

	let searching = $derived(filter.trim() !== '')

	// Browsing, the "Others" list leads with the native database types. Searching, that
	// grouping would outrank the search itself — `ms_sql_server` sorting under `mysql` on
	// "sql" — so the ranked order stands on its own.
	let manualOrderedKeys = $derived(
		!searching
			? [
					...(rankedConnectsManual ?? [])
						.filter((x) => nativeLanguagesCategory.includes(x.key))
						.map((x) => x.key),
					...(rankedConnectsManual ?? [])
						.filter((x) => !nativeLanguagesCategory.includes(x.key))
						.map((x) => x.key)
				]
			: (rankedConnectsManual ?? []).map((x) => x.key)
	)

	let customKeys = $derived(manualOrderedKeys.filter((key) => customResourceTypes.has(key)))
	let otherKeys = $derived(manualOrderedKeys.filter((key) => !customResourceTypes.has(key)))

	// Every row in the order it is rendered, so arrow keys walk the sections as one list.
	// A provider appears in more than one, so rows are addressed by index, not by name.
	let navItems = $derived([
		...customKeys.map((key) => ({ key, oauth: false })),
		...(rankedConnects ?? []).map((x) => ({ key: x.key, oauth: true })),
		...otherKeys.map((key) => ({ key, oauth: false }))
	])
	// Both lists start undefined and render skeletons; "nothing found" only means something
	// once they have landed.
	let listsLoaded = $derived(rankedConnectsManual !== undefined && rankedConnects !== undefined)
	const rowDomId = (index: number) => `resource-type-row-${index}`

	const oauthRowOffset = $derived(customKeys.length)
	const otherRowOffset = $derived(customKeys.length + (rankedConnects?.length ?? 0))

	// Sections are rendered in a fixed order, so the best match is not necessarily the first
	// row: rank the rows against the query to find it.
	function bestMatchIndex(): number {
		let best = navItems.length > 0 ? 0 : -1
		let bestRank = Infinity
		navItems.forEach((item, index) => {
			const rank = resourceTypeMatchRank(item.key, resourceTypeDescriptions[item.key], filter)
			if (rank < bestRank) {
				bestRank = rank
				best = index
			}
		})
		return best
	}

	const highlight = useListHighlight({
		count: () => navItems.length,
		rowId: rowDomId,
		// Sections are rendered in a fixed order, so the best match is not necessarily the
		// first row; Enter should still take the top hit.
		restingIndex: () => (searching ? bestMatchIndex() : -1),
		onActivate: (index) => {
			const item = navItems[index]
			if (!item) return
			item.oauth ? connectOauth(item.key) : selectFromOthers(item.key)
		},
		activateEnterFrom: [SEARCH_INPUT_ID]
	})

	function onListKeydown(e: KeyboardEvent) {
		if (step !== 1) return
		highlight.onKeydown(e)
	}

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
		f={(x) => resourceTypeSearchText(x.key, resourceTypeDescriptions[x.key])}
	/>
	<SearchItems
		{filter}
		items={connectsManual}
		bind:filteredItems={filteredConnectsManual}
		f={(x) => resourceTypeSearchText(x.key, resourceTypeDescriptions[x.key])}
	/>
	{#if step == 1}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- Arrow keys and Enter are caught here so they work whether the search field or a row
		     holds focus. -->
		<!-- Full height so the rows scroll inside their own box: the search field and the sync
		     button stay put, and the drawer itself never scrolls. -->
		<div
			class="flex flex-col h-full min-h-0"
			onkeydown={onListKeydown}
			onpointermove={highlight.pointerMoved}
		>
			<div class="shrink-0 pb-4">
				<div class="relative w-full">
					<Search class="absolute left-2 top-1/2 -translate-y-1/2 text-tertiary" size={14} />
					<TextInput
						bind:this={searchInput}
						inputProps={{ placeholder: 'Search resource type', id: SEARCH_INPUT_ID }}
						bind:value={filter}
						class="pl-7 text-xs w-full"
					/>
				</div>
			</div>

			{#snippet sectionHeading(title: string, count: number)}
				<h2 class="mb-3 text-2xs font-normal uppercase text-secondary">
					{title}{#if searching}<span class="ml-2 text-hint">{count}</span>{/if}
				</h2>
			{/snippet}

			{#snippet resourceButton(key: string, index: number, oauth: boolean)}
				{#snippet icon()}
					<IconedResourceType name={key} silent width="20px" height="20px" />
				{/snippet}
				{#snippet title()}
					<span class="truncate leading-5">{resourceTypeDisplayName(key)}</span>
					<span class="shrink-0 font-mono text-2xs font-normal text-hint">{key}</span>
				{/snippet}
				{#snippet subtitle()}
					{plainDescription(resourceTypeDescriptions[key])}
				{/snippet}
				<!-- `highlighted`: the pointer moves the same highlight the arrow keys move, so
				     the row's own hover is off — two lit rows at once would be ambiguous. -->
				<ListRow
					id={rowDomId(index)}
					aiId={`app-connect-inner-${oauth ? 'oauth-' : ''}${key}`}
					aiDescription={`Connect to ${key}${oauth ? ' with the instance OAuth client' : ''}`}
					{icon}
					{title}
					subtitle={resourceTypeDescriptions[key] ? subtitle : undefined}
					highlighted={index === highlight.index}
					onMouseEnter={() => highlight.hovered(index)}
					onClick={() => (oauth ? connectOauth(key) : selectFromOthers(key))}
				/>
			{/snippet}

			<div class="flex-1 min-h-0 overflow-y-auto">
				{#if searching && listsLoaded && navItems.length === 0}
					<div class="flex flex-col items-center gap-1 py-16 text-center">
						<span class="text-sm text-primary">No resource type matches “{filter.trim()}”</span>
						<span class="text-xs text-secondary">
							Search on the name, the product or what the resource holds — or sync resource types
							with the hub for more.
						</span>
					</div>
				{:else}
					<!-- One gap between sections, owned by the column: a section that a search empties
					     out then takes its spacing with it. -->
					<div class="flex flex-col gap-10">
						{#if customKeys.length > 0}
							<section>
								{@render sectionHeading('Custom resource types', customKeys.length)}
								<div class="flex flex-col gap-0.5">
									{#each customKeys as key, i}
										{@render resourceButton(key, i, false)}
									{/each}
								</div>
							</section>
						{/if}

						{#if !searching || (rankedConnects?.length ?? 0) > 0}
							<section>
								{@render sectionHeading(
									'Instance-configured OAuth APIs',
									rankedConnects?.length ?? 0
								)}
								<div class="flex flex-col gap-0.5">
									{#if rankedConnects}
										{#each rankedConnects as { key }, i}
											{@render resourceButton(key, oauthRowOffset + i, true)}
										{/each}
									{:else}
										{#each new Array(3) as _}
											<Skeleton layout={[[2]]} />
										{/each}
									{/if}
								</div>
								{#if !searching && connects && connects.filter(isSharedConnect).length == 0}
									<div class="text-secondary text-xs w-full"
										>No OAuth APIs have been set up on this instance. To add OAuth APIs, first sync
										the resource types with the hub, then add OAuth configuration. See <a
											href="https://www.windmill.dev/docs/misc/setup_oauth">documentation</a
										>
									</div>
								{/if}
							</section>
						{/if}

						{#if !searching || otherKeys.length > 0}
							<section>
								{@render sectionHeading('Others', otherKeys.length)}

								{#if !searching && connectsManual && connectsManual?.length < 10}
									<div class="text-secondary text-xs p-2">
										Resource types have not been synced with the hub
									</div>
								{/if}

								<div class="flex flex-col gap-0.5">
									{#if rankedConnectsManual}
										{#each otherKeys as key, i}
											{@render resourceButton(key, otherRowOffset + i, false)}
										{/each}
									{:else}
										{#each new Array(9) as _}
											<Skeleton layout={[[2]]} />
										{/each}
									{/if}
								</div>
							</section>
						{/if}
					</div>
				{/if}
			</div>
			<div class="shrink-0 pt-4">
				<SyncResourceTypes
					onSynced={async () => {
						connectsManual = undefined
						await loadResourceTypes()
						connects = undefined
						await loadConnects()
					}}
				/>
			</div>
		</div>
	{:else if step == 2 && manual}
		<div class="flex flex-col gap-4">
			{#if !emptyString(resourceTypeInfo?.description)}
				<GfmMarkdown md={urlize(resourceTypeInfo?.description ?? '', 'md')} prose="sm" noPadding />
			{/if}
			<Label label="Path">
				<ResourcePathHint />
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
				<div class="flex flex-col gap-2">
					<h2 class="text-sm font-semibold text-emphasis">Instructions</h2>
					<ol class="list-decimal pl-5 text-xs text-primary flex flex-col gap-1">
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
					<GfmMarkdown md={description} prose="sm" />
				{/if}
			</div>

			{#if resourceTypeNotFound}
				<div class="flex flex-col gap-2 mb-4">
					<p class="text-red-500 dark:text-red-400 text-xs">
						Resource type '{resourceType}' not found in your workspace
					</p>
					<SyncResourceTypes {resourceType} onSynced={getResourceTypeInfo} />
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
			<!-- The form is a section of its own, not just the next field: it needs more of a break
			from the description than the uniform gap gives. -->
			<div class="mt-2">
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
					<GfmMarkdown
						md={urlize(resourceTypeInfo?.description ?? '', 'md')}
						prose="sm"
						noPadding
					/>
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
							<!-- role=radiogroup: the cards below carry `role="radio"`, which a screen
							     reader can only place ("2 of 2") inside a named group. -->
							<div
								class="flex flex-col gap-2 mb-2"
								role="radiogroup"
								aria-label="How to authenticate"
							>
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
						<OauthScopes bind:scopes options={registryEntry()?.scope_options} />
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
