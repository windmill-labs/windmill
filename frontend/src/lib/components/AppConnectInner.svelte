<script lang="ts">
	import { run } from 'svelte/legacy'

	import { userStore, workspaceStore } from '$lib/stores'
	import IconedResourceType from './IconedResourceType.svelte'
	import {
		OauthService,
		ResourceService,
		VariableService,
		type TokenResponse,
		type ResourceType
	} from '$lib/gen'
	import { emptyString, truncateRev, urlize } from '$lib/utils'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Path from './Path.svelte'
	import { Button, Skeleton } from './common'
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
	import { Pen } from 'lucide-svelte'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import { apiTokenApps, forceSecretValue, linkedSecretValue } from './app_connect'
	import type { SchemaProperty } from '$lib/common'
	import Tooltip from './Tooltip.svelte'

	interface Props {
		step?: number
		resourceType?: string
		isGoogleSignin?: boolean
		disabled?: boolean
		manual?: boolean
		express?: boolean
	}

	let {
		step = $bindable(1),
		resourceType = $bindable(''),
		isGoogleSignin = $bindable(false),
		disabled = $bindable(false),
		manual = $bindable(true),
		express = false
	}: Props = $props()

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
	let connectsManual:
		| [string, { img?: string; instructions: string[]; key?: string }][]
		| undefined = $state(undefined)
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

	let linkedSecret: string | undefined = $state(undefined)
	let linkedSecretCandidates: string[] | undefined = $state(undefined)
	function computeLinkedSecret(resourceType: string, argsKeys: string[], passwords: string[]) {
		linkedSecretCandidates = computeCandidates(resourceType, argsKeys, passwords)
		return (
			forceSecretValue(resourceType) ??
			linkedSecretCandidates?.sort((ua, ub) => linkedSecretValue(ub) - linkedSecretValue(ua))?.[0]
		)
	}

	let scopes: string[] = $state([])
	let extra_params: [string, string][] = []
	let path: string = $state('')
	let description = $state('')

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
	let tokenUrl = $state('')

	let resourceTypeInfo: ResourceType | undefined = $state(undefined)

	let pathError = $state('')

	export async function open(rt?: string) {
		if (!rt) {
			loadResourceTypes()
		}
		step = 1 //express && !manual ? 3 : 1
		value = ''
		description = ''
		resourceType = rt ?? ''
		valueToken = undefined

		// Reset client credentials state
		supportsClientCredentials = false
		useClientCredentials = false
		clientId = ''
		clientSecret = ''
		tokenUrl = ''

		await loadConnects()
		manual = !connects?.includes(resourceType)
		if (manual && express) {
			dispatch('error', 'Express OAuth setup is not available for non OAuth resource types')
			return
		}
		if (rt) {
			if (!manual && express) {
				await getScopesAndParams()
				step = 2
			}
			next()
		}
	}

	async function loadConnects() {
		if (!connects) {
			try {
				connects = (await OauthService.listOauthConnects())
					.filter((x) => x != 'supabase_wizard')
					.sort((a, b) => a.localeCompare(b))
			} catch (e) {
				connects = []
				console.error('Error loading OAuth connects', e)
			}
		}
	}

	const connectAndManual = ['gitlab']

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
						linkedSecret != undefined
					: false)) ||
			step == 3 ||
			(step == 4 && pathError != '') ||
			!isValid
	})

	export async function loadResourceTypes() {
		if (connectsManual) {
			return
		}
		const availableRts = await ResourceService.listResourceTypeNames({
			workspace: $workspaceStore!
		})

		connectsManual = availableRts
			.filter((x) => connectAndManual.includes(x) || !Object.keys(connects ?? {}).includes(x))
			.map(
				(x) =>
					[
						x,
						apiTokenApps[x] ?? {
							instructions: '',
							img: undefined,
							linkedSecret: undefined
						}
					] as [string, { img?: string; instructions: string[]; key?: string }]
			)
			.sort((a, b) => a[0].localeCompare(b[0]))
		const filteredNativeLanguages = filteredConnectsManual?.filter(
			(o) => nativeLanguagesCategory?.includes(o[0]) ?? false
		)

		try {
			filteredConnectsManual = [
				...(filteredNativeLanguages ?? []),
				...(filteredConnectsManual ?? []).filter(
					([key, _]) => !nativeLanguagesCategory.includes(key)
				)
			]
		} catch (e) {}
	}

	function popupListener(event) {
		console.log('Received oauth popup message', event)
		let data = event.data
		if (event.origin == null || event.origin !== window.location.origin) {
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

	function processPopupData(data) {
		console.log('Processing oauth popup data')
		if (data.type === 'error') {
			sendUserToast(data.error, true)
			step = 2
		} else if (data.type === 'success') {
			resourceType = data.resource_type
			value = data.res.access_token!
			valueToken = data.res
			step = 4
			if (express) {
				path = `u/${$userStore?.username}/${resourceType}_${new Date().getTime()}`
				next()
			}
		}
	}

	async function getScopesAndParams() {
		const connect = await OauthService.getOauthConnect({ client: resourceType })
		scopes = connect.scopes ?? []
		extra_params = Object.entries(connect.extra_params ?? {}) as [string, string][]

		/**
		 * Check if the OAuth provider supports client_credentials grant type
		 * This determines whether to show the OAuth flow selection UI
		 */
		supportsClientCredentials = connect.grant_types?.includes('client_credentials') ?? false
	}

	async function getResourceTypeInfo() {
		resourceTypeInfo = await ResourceService.getResourceType({
			workspace: $workspaceStore!,
			path: resourceType
		})
		const props: Record<string, SchemaProperty> = resourceTypeInfo?.schema?.['properties'] ?? {}
		const newArgsKeys = Object.keys(props) ?? []
		const passwords = newArgsKeys.filter((x) => {
			return props?.[x]?.password
		})
		if (!linkedSecret) {
			linkedSecret = computeLinkedSecret(resourceType, newArgsKeys, passwords)
		}
	}
	export async function next() {
		if (step == 1) {
			linkedSecret = undefined
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
				 * No popup window or user interaction required
				 * Uses instance-level OAuth credentials for server-to-server auth
				 */
				try {
					// Trim whitespace from credentials to avoid false negatives
					const trimmedClientId = clientId.trim()
					const trimmedClientSecret = clientSecret.trim()

					// Validate required fields
					if (!trimmedClientId || !trimmedClientSecret) {
						sendUserToast(
							'Client ID and Client Secret are required for client credentials flow',
							true
						)
						return
					}

					const requestBody: any = {
						scopes: scopes,
						cc_client_id: trimmedClientId,
						cc_client_secret: trimmedClientSecret
					}

					// Add token URL override if provided
					if (tokenUrl.trim()) {
						requestBody.cc_token_url = tokenUrl.trim()
					}

					const tokenResponse = await OauthService.connectClientCredentials({
						client: resourceType,
						requestBody
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
				const url = new URL(`/api/oauth/connect/${resourceType}`, window.location.origin)
				url.searchParams.append('scopes', scopes.join('+'))
				if (extra_params.length > 0) {
					extra_params.forEach(([key, value]) => url.searchParams.append(key, value))
				}
				window.addEventListener('message', popupListener)
				window.addEventListener('storage', handleStorageEvent)
				window.open(url.toString(), '_blank', 'popup=true')
				step += 1
			}
		} else {
			if (!path) {
				throw Error('Path is not set')
			}
			let exists = await VariableService.existsVariable({
				workspace: $workspaceStore!,
				path
			})
			if (exists) {
				throw Error(`Variable at path ${path} already exists. Delete it or pick another path`)
			}
			exists = await ResourceService.existsResource({
				workspace: $workspaceStore!,
				path
			})

			if (exists) {
				throw Error(`Resource at path ${path} already exists. Delete it or pick another path`)
			}

			if (resourceType == 'snowflake_oauth') {
				const account_identifier = extra_params.find(([key, _]) => key == 'account_identifier')
				if (account_identifier) {
					args['account_identifier'] = account_identifier[1]
				}
			}

			let account: number | undefined = undefined
			if (valueToken?.expires_in != undefined) {
				const accountData: any = {
					refresh_token: valueToken.refresh_token ?? '',
					expires_in: valueToken.expires_in,
					client: resourceType,
					grant_type: valueToken.grant_type || 'authorization_code'
				}

				// Add client credentials if using client_credentials flow
				if (useClientCredentials) {
					accountData.cc_client_id = clientId.trim()
					accountData.cc_client_secret = clientSecret.trim()
					// Add token URL override if provided
					if (tokenUrl.trim()) {
						accountData.cc_token_url = tokenUrl.trim()
					}
				}

				account = Number(
					await OauthService.createAccount({
						workspace: $workspaceStore!,
						requestBody: accountData
					})
				)
			}

			const resourceValue = args

			let saveVariable = false
			if (!manual || linkedSecret != undefined) {
				let v = manual ? args[linkedSecret ?? ''] : value
				if (typeof v == 'string' && v != '' && !v.startsWith('$var:')) {
					saveVariable = true
					await VariableService.createVariable({
						workspace: $workspaceStore!,
						requestBody: {
							path,
							value: v,
							is_secret: true,
							description: emptyString(description)
								? `${manual ? 'Token' : 'OAuth token'} for ${resourceType}`
								: description,
							is_oauth: !manual,
							account: account
						}
					})
					resourceValue[linkedSecret ?? 'token'] = `$var:${path}`
				}
			}

			await ResourceService.createResource({
				workspace: $workspaceStore!,
				requestBody: {
					resource_type: resourceType,
					path,
					value: resourceValue,
					description
				}
			})
			dispatch('refresh', path)
			dispatch('close')
			sendUserToast(`Saved resource${saveVariable ? ' and variable' : ''} path: ${path}`)
			step = 1
			resourceType = ''
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
	let filteredConnectsManual: [string, { img?: string; instructions: string[]; key?: string }][] =
		$state([])

	let editScopes = $state(false)
</script>

{#if !express}
	<SearchItems
		{filter}
		items={connects
			? connects.map((key) => ({
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
		f={(x) => x[0]}
	/>
	{#if step == 1}
		<div class="w-12/12 pb-2 flex flex-row my-1 gap-1">
			<input
				type="text"
				placeholder="Search resource type"
				bind:value={filter}
				class="text-2xl grow"
			/>
		</div>

		<h2 class="mb-4">OAuth APIs</h2>
		<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
			{#if filteredConnects}
				{#each filteredConnects as { key }}
					<Button
						size="sm"
						variant="border"
						color={key === resourceType ? 'blue' : 'light'}
						btnClasses={key === resourceType ? '!border-2' : 'm-[1px]'}
						on:click={() => {
							manual = false
							resourceType = key
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
		{#if connects && connects.length == 0}
			<div class="text-secondary text-sm w-full"
				>No OAuth APIs has been setup on the instance. To add oauth APIs, first sync the resource
				types with the hub, then add oauth configuration. See <a
					href="https://www.windmill.dev/docs/misc/setup_oauth">documentation</a
				>
			</div>
		{/if}

		<h2 class="mt-8 mb-4">Others</h2>

		{#if connectsManual && connectsManual?.length < 10}
			<div class="text-secondary p-2">
				Resource Types have not been synced with the hub. Go to the admins workspace to sync them
				(and add a schedule to do daily):
				<p class="mt-4"
					>1. Go to the "admins" workspaces:
					<img src="{base}/sync_resource_types.png" alt="sync resource types" class="mt-2" />
				</p>
				<p class="mt-4">
					2: Run the synchronization script:
					<img src="{base}/sync_resource_types2.png" alt="sync resource types" class="mt-2" />
				</p>
			</div>
		{/if}

		<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
			{#if filteredConnectsManual}
				{#each filteredConnectsManual as [key, _]}
					{#if nativeLanguagesCategory.includes(key)}
						<Button
							size="sm"
							variant="border"
							color={key === resourceType ? 'blue' : 'light'}
							btnClasses={key === resourceType ? '!border-2' : 'm-[1px]'}
							on:click={() => {
								manual = true
								resourceType = key
								next()
							}}
						>
							<IconedResourceType name={key} after={true} width="20px" height="20px" />
						</Button>
					{/if}
				{/each}
			{/if}
		</div>

		<div class="mt-8 mb-4"></div>
		<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
			{#if filteredConnectsManual}
				{#each filteredConnectsManual as [key, _]}
					{#if !nativeLanguagesCategory.includes(key)}
						<!-- Exclude specific items -->
						<Button
							aiId={`app-connect-inner-${key}`}
							aiDescription={`Connect to ${key}`}
							size="sm"
							variant="border"
							color={key === resourceType ? 'blue' : 'light'}
							btnClasses={key === resourceType ? '!border-2' : 'm-[1px]'}
							on:click={() => {
								manual = true
								resourceType = key
								next()
							}}
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
	{:else if step == 2 && manual}
		<Path
			bind:error={pathError}
			bind:path
			initialPath=""
			namePlaceholder={resourceType}
			kind="resource"
		/>

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
					<img class="m-auto max-h-60" alt="connect" src={base + apiTokenApps[resourceType].img} />
				</div>
			{/if}
		{:else if !emptyString(resourceTypeInfo?.description)}
			<h4 class="mt-8 mb-2">{resourceTypeInfo?.name} description</h4>
			<div class="text-sm">
				<Markdown md={urlize(resourceTypeInfo?.description ?? '', 'md')} />
			</div>
		{/if}
		{#if resourceType == 'postgresql' || resourceType == 'mysql' || resourceType == 'mongodb'}
			<WhitelistIp />
		{/if}

		<h4 class="mt-8 inline-flex items-center gap-4"
			>Resource description <Required required={false} />
			<div class="flex gap-1 items-center">
				<Toggle size="xs" bind:checked={renderDescription} />
				<Pen size={14} />
			</div>
		</h4>
		{#if renderDescription}
			<div>
				<div class="flex flex-row-reverse text-2xs text-tertiary -mt-1">GH Markdown</div>
				<textarea use:autosize bind:value={description} placeholder={'Resource description'}
				></textarea>
			</div>
		{:else if description == undefined || description == ''}
			<div class="text-sm text-tertiary">No description provided</div>
		{:else}
			<div class="mt-2"></div>
			<GfmMarkdown md={description} />
		{/if}
		<div class="mt-12">
			{#key resourceTypeInfo}
				<ApiConnectForm
					bind:linkedSecret
					bind:description
					{linkedSecretCandidates}
					{resourceType}
					{resourceTypeInfo}
					bind:args
					bind:isValid
				/>
			{/key}
		</div>
	{:else if step == 2 && !manual}
		{#if manual == false && resourceType != ''}
			<h1 class="mb-4">{resourceType}</h1>
			<div class="my-4 text-secondary"
				>Create a resource backed by an OAuth connection, whose token is fetched from the external
				services and refreshed automatically if needed before expiration.</div
			>
			<h4 class="mb-2">Description</h4>
			<div class="text-sm mb-8">
				<Markdown md={urlize(resourceTypeInfo?.description ?? '', 'md')} />
			</div>

			{#if supportsClientCredentials}
				<div style="margin-bottom: 16px;">
					<h3 class="mb-4">Authentication Method</h3>
					<div style="display: flex; align-items: center; gap: 8px;">
						<input
							type="checkbox"
							style="width: 16px; height: 16px; margin: 0;"
							bind:checked={useClientCredentials}
						/>
						<span style="font-size: 14px; font-weight: 600;">Use Client Credentials Flow</span>
						<Tooltip>
							Server-to-server authentication without user interaction.
							<br /><br />
							Provide your own OAuth client credentials for this resource.
						</Tooltip>
					</div>

					{#if useClientCredentials}
						<div style="margin-top: 16px;">
							<label style="display: block; margin-bottom: 8px;">
								<span style="font-weight: 600;">Client ID</span>
								<input
									type="text"
									bind:value={clientId}
									placeholder="Enter OAuth client ID"
									class="w-full p-2 border border-gray-300 rounded mt-1"
									required
								/>
							</label>
							<label style="display: block;">
								<span style="font-weight: 600;">Client Secret</span>
								<input
									type="password"
									bind:value={clientSecret}
									placeholder="Enter OAuth client secret"
									class="w-full p-2 border border-gray-300 rounded mt-1"
									required
								/>
							</label>
							<label style="display: block; margin-top: 8px;">
								<span style="font-weight: 600;">Token URL Override (Optional)</span>
								<input
									type="url"
									bind:value={tokenUrl}
									placeholder="Custom token endpoint URL"
									class="w-full p-2 border border-gray-300 rounded mt-1"
								/>
								<div style="font-size: 12px; color: #666; margin-top: 4px;">
									Override the instance-level token URL for this resource
								</div>
							</label>
						</div>
					{/if}
				</div>
			{/if}

			<h3 class="mb-4 flex gap-4"
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
		{/if}
	{:else if step == 3 && !manual && !express}
		{#if useClientCredentials}
			Connecting with client credentials...
		{:else}
			Finish connection in popup window
		{/if}
	{:else}
		<Path
			initialPath=""
			namePlaceholder={resourceType}
			bind:error={pathError}
			bind:path
			kind="resource"
		/>
		{#if apiTokenApps[resourceType] || !manual}
			<ul class="mt-10">
				<li>
					1. A secret variable containing the {apiTokenApps[resourceType]?.linkedSecret ?? 'token'}
					<span class="font-bold">{truncateRev(value, 5, '*****')}</span>
					will be stored a
					<span class="font-mono whitespace-nowrap">{path}</span>.
				</li>
				<li class="mt-4">
					2. The resource containing that token will be stored at the same path <span
						class="font-mono whitespace-nowrap">{path}</span
					>. The Variable and Resource will be "linked together", they will be deleted and renamed
					together.
				</li></ul
			>
		{/if}
	{/if}
{/if}
