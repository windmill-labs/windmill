<script lang="ts" context="module">
	const apiTokenApps: Record<
		string,
		{ img?: string; instructions: string[]; linkedSecret?: string }
	> = {
		airtable: {
			img: '/airtable_connect.png',
			instructions: [
				'Click to <a href="https://airtable.com/create/tokens" target="_blank" rel=”noopener noreferrer”>https://airtable.com/create/tokens</a>',
				'Click on "Create new token"',
				'Set a name, specify the scopes or the access level and click on "Create token"',
				'Copy the token'
			]
		},
		discord_webhook: {
			img: '/discord_webhook.png',
			instructions: ['Click on Server Settings', 'Click on Integration', 'Find "Webhooks"'],
			linkedSecret: 'webhook_url'
		},
		toggl: {
			img: '/toggl_connect.png',
			instructions: [
				'Go to <a href="https://track.toggl.com/profile" target="_blank" rel=”noopener noreferrer”>https://track.toggl.com/profile</a>',
				'Find "API Token"'
			]
		},
		mailchimp: {
			img: '/mailchimp_connect.png',
			instructions: [
				'Go to <a href="https://admin.mailchimp.com/account/api" target="_blank" rel=”noopener noreferrer”>https://admin.mailchimp.com/account/api</a>',
				'Find "Your API Keys"'
			]
		},
		sendgrid: {
			img: '/sendgrid_connect.png',
			instructions: [
				'Go to <a href="https://app.sendgrid.com/settings/api_keys" target="_blank" rel=”noopener noreferrer”>https://app.sendgrid.com/settings/api_keys</a>',
				'Create an API key',
				'Copy your key'
			]
		},
		supabase: {
			img: '/supabase_connect.png',
			instructions: ['Go to the API Settings of your app to find the project URL and key']
		},

		square: {
			img: '/square_connect.gif',
			instructions: [
				'Go to <a href="https://developer.squareup.com/apps" target="_blank" rel=”noopener noreferrer”>https://developer.squareup.com/apps</a>',
				'In the left pane, choose Credentials',
				'At the top of the page, choose Production mode for a production access token or Sandbox mode for a Sandbox access token.'
			]
		}
	}
</script>

<script lang="ts">
	import { oauthStore, workspaceStore } from '$lib/stores'
	import IconedResourceType from './IconedResourceType.svelte'
	import { OauthService, ResourceService, VariableService, type TokenResponse } from '$lib/gen'
	import { emptyString, truncateRev } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Path from './Path.svelte'
	import { Button, Drawer, Skeleton } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ApiConnectForm from './ApiConnectForm.svelte'
	import SearchItems from './SearchItems.svelte'
	import autosize from 'svelte-autosize'
	import WhitelistIp from './WhitelistIp.svelte'
	import { sendUserToast } from '$lib/toast'
	import OauthScopes from './OauthScopes.svelte'

	export let newPageOAuth = false

	const nativeLanguagesCategory = [
		'postgresql',
		'mysql',
		'bigquery',
		'snowflake',
		'mssql',
		'graphql'
	]

	let filter = ''
	let manual = false
	let value: string = ''
	let valueToken: TokenResponse
	let connects:
		| Record<string, { scopes: string[]; extra_params?: Record<string, string> }>
		| undefined = undefined
	let connectsManual:
		| [string, { img?: string; instructions: string[]; key?: string }][]
		| undefined = undefined
	let args: any = {}

	$: linkedSecretCandidates = apiTokenApps[resource_type]?.linkedSecret
		? ([apiTokenApps[resource_type]?.linkedSecret] as string[])
		: args != undefined
		? Object.keys(args).filter((x) =>
				['token', 'secret', 'key', 'pass', 'private'].some((y) => x.toLowerCase().includes(y))
		  )
		: undefined

	function linkedSecretValue(x: string) {
		let r = 0
		if (x.includes('secret')) {
			r += 10
		}
		if (x.includes('password')) {
			r += 5
		}
		if (x.includes('private')) {
			r += 4
		}
		if (x.includes('key')) {
			r += 3
		}
		if (x.includes('token')) {
			r += 2
		}
		if (x.includes('pass')) {
			r += 1
		}
		return r
	}
	$: linkedSecret = linkedSecretCandidates?.sort(
		(ua, ub) => linkedSecretValue(ua) - linkedSecretValue(ub)
	)?.[0]

	let scopes: string[] = []
	let extra_params: [string, string][] = []

	let path: string
	let description = ''

	let drawer: Drawer
	let resource_type = ''
	let step = 1

	let no_back = false

	let pathError = ''

	export async function open(rt?: string) {
		step = 1
		value = ''
		description = ''
		no_back = false
		resource_type = rt ?? ''

		await loadConnects()

		const connect = connects?.[resource_type]
		if (connect) {
			scopes = connect.scopes
			extra_params = Object.entries(connect.extra_params ?? {})
		} else {
			manual = true
			if (rt) {
				next()
			}
		}
		drawer.openDrawer?.()
	}

	export function openFromOauth(rt: string) {
		resource_type = rt
		value = $oauthStore?.access_token!
		valueToken = $oauthStore!
		$oauthStore = undefined
		manual = false
		step = 3
		no_back = true
		drawer.openDrawer?.()
	}

	async function loadConnects() {
		const nconnects = await OauthService.listOAuthConnects()
		if (nconnects['supabase_wizard']) {
			delete nconnects['supabase_wizard']
		}
		connects = nconnects
	}

	const connectAndManual = ['gitlab']

	async function loadResources() {
		await loadConnects()
		const availableRts = await ResourceService.listResourceTypeNames({
			workspace: $workspaceStore!
		})

		connectsManual = availableRts
			.filter((x) => connectAndManual.includes(x) || !Object.keys(connects ?? {}).includes(x))
			.map((x) => [
				x,
				apiTokenApps[x] ?? {
					instructions: '',
					img: undefined,
					linkedSecret: undefined
				}
			])
		const filteredNativeLanguages = filteredConnectsManual?.filter(([key, _]) =>
			nativeLanguagesCategory.includes(key)
		)

		filteredConnectsManual = [
			...(filteredNativeLanguages ?? []),
			...(filteredConnectsManual ?? []).filter(([key, _]) => !nativeLanguagesCategory.includes(key))
		]
	}

	async function next() {
		if (step == 1 && manual) {
			step += 1
			args = {}
		} else if (step == 1 && !manual) {
			const url = new URL(`/api/oauth/connect/${resource_type}`, window.location.origin)
			url.searchParams.append('scopes', scopes.join('+'))
			if (extra_params.length > 0) {
				extra_params.forEach(([key, value]) => url.searchParams.append(key, value))
			}
			if (!newPageOAuth) {
				window.location.href = url.toString()
			} else {
				window.open(url.toString(), '_blank')
				drawer.closeDrawer()
			}
		} else {
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

			let account: number | undefined = undefined
			if (valueToken?.expires_in != undefined) {
				account = Number(
					await OauthService.createAccount({
						workspace: $workspaceStore!,
						requestBody: {
							refresh_token: valueToken.refresh_token ?? '',
							expires_in: valueToken.expires_in,
							owner: path.split('/').slice(0, 2).join('/'),
							client: resource_type
						}
					})
				)
			}

			const resourceValue = args

			if (!manual || linkedSecret != undefined) {
				await VariableService.createVariable({
					workspace: $workspaceStore!,
					requestBody: {
						path,
						value: manual ? args[linkedSecret ?? ''] : value,
						is_secret: true,
						description: emptyString(description)
							? `${manual ? 'Token' : 'OAuth token'} for ${resource_type}`
							: description,
						is_oauth: !manual,
						account: account
					}
				})
				resourceValue[linkedSecret ?? 'token'] = `$var:${path}`
			}

			await ResourceService.createResource({
				workspace: $workspaceStore!,
				requestBody: {
					resource_type,
					path,
					value: resourceValue,
					description
				}
			})
			dispatch('refresh', path)
			sendUserToast(`App token set at resource and variable path: ${path}`)
			drawer.closeDrawer?.()
		}
	}

	async function back() {
		if (step > 1) {
			step -= 1
		}
	}

	const dispatch = createEventDispatcher()

	$: isGoogleSignin =
		step == 1 &&
		(resource_type == 'google' ||
			resource_type == 'gmail' ||
			resource_type == 'gcal' ||
			resource_type == 'gdrive' ||
			resource_type == 'gsheets')

	$: disabled =
		(step == 1 && resource_type == '') ||
		(step == 2 &&
			value == '' &&
			args &&
			args['token'] == '' &&
			args['password'] == '' &&
			args['api_key'] == '' &&
			args['key'] == '' &&
			linkedSecret != undefined) ||
		(step == 3 && pathError != '') ||
		!isValid

	let isValid = true
	let filteredConnects: [string, { scopes: string[]; extra_params?: Record<string, string> }][] = []
	let filteredConnectsManual: [string, { img?: string; instructions: string[]; key?: string }][] =
		[]
</script>

<SearchItems
	{filter}
	items={connects ? Object.entries(connects).sort((a, b) => a[0].localeCompare(b[0])) : undefined}
	bind:filteredItems={filteredConnects}
	f={(x) => x[0]}
/>
<SearchItems
	{filter}
	items={connectsManual?.sort((a, b) => a[0].localeCompare(b[0]))}
	bind:filteredItems={filteredConnectsManual}
	f={(x) => x[0]}
/>

<Drawer
	bind:this={drawer}
	on:close={() => {
		dispatch('close')
	}}
	on:open={() => {
		loadResources()
	}}
	size="800px"
>
	<DrawerContent
		title="Add a Resource"
		on:close={drawer.closeDrawer}
		tooltip="Resources represent connections to third party systems. Learn more on how to integrate external APIs."
		documentationLink="https://www.windmill.dev/docs/integrations/integrations_on_windmill"
	>
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
					{#each filteredConnects as [key, values]}
						<Button
							size="sm"
							variant="border"
							color={key === resource_type ? 'blue' : 'light'}
							btnClasses={key === resource_type ? '!border-2' : 'm-[1px]'}
							on:click={() => {
								manual = false
								resource_type = key
								scopes = values.scopes
								extra_params = Object.entries(values.extra_params ?? {})
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
			{#if connects && Object.keys(connects).length == 0}
				<div class="text-secondary text-sm w-full"
					>No OAuth APIs has been setup on the instance. To add oauth APIs, first sync the resource
					types with the hub, then add oauth configuration. See <a
						href="https://www.windmill.dev/docs/misc/setup_oauth">documentation</a
					>
				</div>
			{/if}
			{#if manual == false && resource_type != ''}
				<h3>Scopes</h3>
				{#if !manual && resource_type != ''}
					<OauthScopes bind:scopes />
				{/if}
			{/if}

			<h2 class="mt-8 mb-4">Others</h2>
			<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
				{#if filteredConnectsManual}
					{#each filteredConnectsManual as [key, _]}
						{#if nativeLanguagesCategory.includes(key)}
							<Button
								size="sm"
								variant="border"
								color={key === resource_type ? 'blue' : 'light'}
								btnClasses={key === resource_type ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
								on:click={() => {
									manual = true
									resource_type = key
									next()
									dispatch('click')
								}}
							>
								<IconedResourceType name={key} after={true} width="20px" height="20px" />
							</Button>
						{/if}
					{/each}
				{/if}
			</div>

			<h2 class="mt-8 mb-4" />
			<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
				{#if filteredConnectsManual}
					{#each filteredConnectsManual as [key, _]}
						{#if !nativeLanguagesCategory.includes(key)}
							<!-- Exclude specific items -->
							<Button
								size="sm"
								variant="border"
								color={key === resource_type ? 'blue' : 'light'}
								btnClasses={key === resource_type ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
								on:click={() => {
									manual = true
									resource_type = key
									next()
									dispatch('click')
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
				namePlaceholder={resource_type}
				kind="resource"
			/>

			{#if apiTokenApps[resource_type]}
				<h2 class="mt-4 mb-2">Instructions</h2>
				<div class="pl-10">
					<ol class="list-decimal">
						{#each apiTokenApps[resource_type].instructions as step}
							<li>
								{@html step}
							</li>
						{/each}
					</ol>
				</div>
				{#if apiTokenApps[resource_type].img}
					<div class="mt-4 w-full overflow-hidden">
						<img class="m-auto max-h-60" alt="connect" src={apiTokenApps[resource_type].img} />
					</div>
				{/if}
			{/if}
			{#if resource_type == 'postgresql' || resource_type == 'mysql' || resource_type == 'mongodb'}
				<WhitelistIp />
			{/if}

			<h2 class="mt-4">Value</h2>
			<div class="mt-4">
				{#key resource_type}
					<ApiConnectForm
						{linkedSecret}
						{linkedSecretCandidates}
						{resource_type}
						bind:args
						bind:isValid
					/>
				{/key}
			</div>

			<h2 class="mt-4 mb-2">Description</h2>
			<textarea type="text" autocomplete="off" use:autosize bind:value={description} />
		{:else}
			<Path
				initialPath=""
				namePlaceholder={resource_type}
				bind:error={pathError}
				bind:path
				kind="resource"
			/>
			{#if apiTokenApps[resource_type] || !manual}
				<ul class="mt-10">
					<li>
						1. A secret variable containing the {apiTokenApps[resource_type]?.linkedSecret ??
							'token'}
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
		<div slot="actions" class="flex gap-1">
			{#if step > 1 && !no_back}
				<Button variant="border" on:click={back}>Back</Button>
			{/if}
			{#if isGoogleSignin}
				<button {disabled} on:click={next}>
					<img class="h-10 w-auto" src="/google_signin.png" alt="Google sign-in" />
				</button>
			{:else}
				<Button {disabled} on:click={next}>
					{#if step == 1 && !manual}
						Connect
					{:else if step == 1 && manual}
						Next
					{:else}
						Save
					{/if}
				</Button>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
