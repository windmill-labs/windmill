<script lang="ts" context="module">
	const apiTokenApps: Record<string, { img?: string; instructions: string; key?: string }> = {
		airtable: {
			img: 'airtable_connect.png',
			instructions: 'Click on the top-right avatar -> Account -> Api'
		},
		discord_webhook: {
			img: 'discord_webhook.png',
			instructions: 'Server Settings -> Integration -> Webhooks',
			key: 'webhook_url'
		},
		toggl: {
			img: 'toggl_connect.png',
			instructions: 'Go to https://track.toggl.com/profile -> API Token'
		}
	}
</script>

<script lang="ts">
	import { oauthStore, userStore, workspaceStore } from '$lib/stores'
	import { faMinus, faPlus } from '@fortawesome/free-solid-svg-icons'
	import IconedResourceType from './IconedResourceType.svelte'
	import { OauthService, ResourceService, VariableService, type TokenResponse } from '$lib/gen'
	import { page } from '$app/stores'
	import { sendUserToast, truncateRev } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import Password from './Password.svelte'
	import Path from './Path.svelte'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'

	let manual = false
	let value: string = ''
	let valueToken: TokenResponse
	let connects: Record<string, { scopes: string[]; extra_params?: Record<string, string> }> = {}
	let connectsManual: [string, { img?: string; instructions: string; key?: string }][] = []
	let key: string = 'token'

	$: key = apiTokenApps[resource_type]?.key ?? 'token'

	let scopes: string[] = []
	let extra_params: [string, string][] = []

	let path: string

	let drawer: Drawer
	let resource_type = ''
	let step = 1

	let no_back = false

	let pathError = ''

	export async function open(rt?: string) {
		step = 1
		value = ''
		no_back = false
		resource_type = rt ?? ''

		await loadConnects()

		const connect = connects[resource_type]
		if (connect) {
			scopes = connect.scopes
			extra_params = Object.entries(connect.extra_params ?? {})
		}
		drawer.openDrawer()
	}

	export function openFromOauth(rt: string) {
		resource_type = rt
		value = $oauthStore?.access_token!
		valueToken = $oauthStore!
		$oauthStore = undefined
		manual = false
		step = 3
		no_back = true
		drawer.openDrawer()
	}

	async function loadConnects() {
		connects = await OauthService.listOAuthConnects()
	}

	async function loadResources() {
		const availableRts = await ResourceService.listResourceTypeNames({
			workspace: $workspaceStore!
		})
		connectsManual = Object.entries(apiTokenApps).filter(([key, _]) => availableRts.includes(key))
	}

	async function next() {
		if (step < 3 && manual) {
			step += 1
		} else if (step == 1 && !manual) {
			const url = new URL(`/api/oauth/connect/${resource_type}`, $page.url.origin)
			url.searchParams.append('scopes', scopes.join('+'))
			if (extra_params.length > 0) {
				extra_params.forEach(([key, value]) => url.searchParams.append(key, value))
			}
			window.location.href = url.toString()
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
			if (valueToken?.refresh_token != undefined && valueToken?.expires_in != undefined) {
				account = Number(
					await OauthService.createAccount({
						workspace: $workspaceStore!,
						requestBody: {
							refresh_token: valueToken.refresh_token!,
							expires_in: valueToken.expires_in!,
							owner: path.split('/').slice(0, 2).join('/'),
							client: resource_type
						}
					})
				)
			}
			const description = `${manual ? 'Token' : 'OAuth token'} for ${resource_type}`
			await VariableService.createVariable({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					value,
					is_secret: true,
					description,
					is_oauth: !manual,
					account: account
				}
			})
			const resourceValue = {}
			resourceValue[key] = `$var:${path}`
			await ResourceService.createResource({
				workspace: $workspaceStore!,
				requestBody: {
					resource_type,
					path,
					value: resourceValue,
					description,
					is_oauth: !manual
				}
			})
			dispatch('refresh')
			sendUserToast(`App token set at resource and variable path: ${path}`)
			drawer.closeDrawer()
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
		(step == 2 && value == '') ||
		(step == 3 && pathError != '')
</script>

<Drawer
	bind:this={drawer}
	on:close={() => {
		dispatch('close')
	}}
	on:open={() => {
		loadConnects()
		loadResources()
	}}
>
	<DrawerContent title="Connect an API" on:close={drawer.closeDrawer}>
		{#if step == 1}
			{#if resource_type && !connects[resource_type] && !connectsManual.find((x) => x[0] == resource_type)}
				<Alert class="mb-4" type="error" title="Resource type not found">
					The resource type "{resource_type}" seems to not have an OAuth API integration. You can
					still create this resource manually by closing this modal and pressing: "Add a resource".
					You can also contribute to windmill and add it as an API integration if relevant.
				</Alert>
			{/if}
			<h2 class="mb-2">OAuth APIs</h2>
			{#if Object.keys(connects).length == 0}
				<Alert class="mb-4" type="error" title="No OAuth connection setup">
					Your instance has no OAuth connection setup. You can still add a resource manually.
				</Alert>
			{/if}
			<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
				{#each Object.entries(connects).sort((a, b) => a[0].localeCompare(b[0])) as [key, values]}
					<Button
						size="sm"
						variant="border"
						color={key === resource_type ? 'blue' : 'dark'}
						btnClasses={key === resource_type ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						on:click={() => {
							manual = false
							resource_type = key
							scopes = values.scopes
							extra_params = Object.entries(values.extra_params ?? {})

							dispatch('click')
						}}
					>
						<IconedResourceType name={key} after={true} width="20px" height="20px" />
					</Button>
				{/each}
			</div>
			<h3>Scopes</h3>
			{#if !manual && resource_type != ''}
				{#each scopes as v}
					<div class="flex flex-row max-w-md mb-2">
						<input type="text" bind:value={v} />
						<Button
							variant="border"
							color="red"
							size="xs"
							btnClasses="mx-6"
							on:click={() => {
								scopes = scopes.filter((el) => el != v)
							}}
						>
							<Icon data={faMinus} />
						</Button>
					</div>
				{/each}
				<div class="flex items-center mt-1">
					<Button
						variant="border"
						color="blue"
						size="sm"
						endIcon={{ icon: faPlus }}
						on:click={() => {
							scopes = scopes.concat('')
						}}
					>
						Add item
					</Button>
					<span class="ml-2 text-sm text-gray-500">
						({(scopes ?? []).length} item{(scopes ?? []).length > 1 ? 's' : ''})
					</span>
				</div>
			{:else}
				<p class="italic text-sm">Pick an OAuth API and customize the scopes here</p>
			{/if}
			<h2 class="mt-8 mb-2">Non OAuth APIs</h2>
			{#if Object.keys(connectsManual).length == 0}
				<Alert class="mb-4" type="error" title="No resource types synced">
					Your instance has no resource types setup. Sync with the hub to get all the latest
					resource types.
				</Alert>
			{/if}
			<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
				{#each connectsManual as [key, instructions]}
					<Button
						size="sm"
						variant="border"
						color={key === resource_type ? 'blue' : 'dark'}
						btnClasses={key === resource_type ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						on:click={() => {
							manual = true
							resource_type = key
							dispatch('click')
						}}
					>
						<IconedResourceType name={key} after={true} width="20px" height="20px" />
					</Button>
				{/each}
			</div>
		{:else if step == 2}
			{#if manual}
				<div class="mb-1 font-semibold text-gray-700 mt-6">Instructions</div>
				<div>
					{apiTokenApps[resource_type].instructions}
				</div>
				{#if apiTokenApps[resource_type].img}
					<div class="mt-4">
						<img alt="connect" src={apiTokenApps[resource_type].img} />
					</div>
				{/if}
				<div class="mt-4">
					<Password bind:password={value} label="Paste token here" />
				</div>
			{/if}
		{:else}
			<Path
				bind:error={pathError}
				bind:path
				initialPath={`u/${$userStore?.username ?? ''}/my_${resource_type}`}
				kind="resource"
			/>
			<ul class="mt-10 bg-white">
				<li>
					1. A secret variable containing the token <span class="font-bold"
						>{truncateRev(value, 5, '*****')}</span
					>
					will be stored at
					<span class="font-mono">{path}</span>. You can refer to this variable anywhere this token
					is required.
				</li>
				<li class="mt-4">
					2. A resource with a unique token field will be stored at <span class="font-mono"
						>{path}</span
					>
					and refer to the secret variable <span class="font-mono">{path}</span> as its token (using
					variable templating
					<span class="font-mono">`$var:${path}`</span>). You can refer to this resource anywhere
					this token is required. A script can use the resource type
					<span class="font-mono">{resource_type}</span> as a type parameter to restrict the kind of
					tokens it accepts to this api.
				</li>
			</ul>
		{/if}
		<div slot="submission" class="flex items-center gap-4">
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
					{:else if step == 3}
						Add resource
					{:else}
						Next
					{/if}
				</Button>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
