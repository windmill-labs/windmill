<script lang="ts">
	import { oauthStore, workspaceStore } from '$lib/stores'
	import IconedResourceType from './IconedResourceType.svelte'
	import {
		OauthService,
		ResourceService,
		VariableService,
		type TokenResponse,
		type ResourceType
	} from '$lib/gen'
	import { emptyString, truncateRev, urlize } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Path from './Path.svelte'
	import { Button, Drawer, Skeleton } from './common'
	import ApiConnectForm from './ApiConnectForm.svelte'
	import SearchItems from './SearchItems.svelte'
	import WhitelistIp from './WhitelistIp.svelte'
	import { sendUserToast } from '$lib/toast'
	import OauthScopes from './OauthScopes.svelte'
	import Markdown from 'svelte-exmarkdown'
	import autosize from '$lib/autosize'
	import Required from './Required.svelte'
	import Toggle from './Toggle.svelte'
	import { Pen } from 'lucide-svelte'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import { apiTokenApps, forceSecretValue, linkedSecretValue } from './app_connect'

	export let newPageOAuth = false
	export let isValid = true
	export let step = 1

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
	let valueToken: TokenResponse | undefined = undefined
	let connects:
		| Record<string, { scopes: string[]; extra_params?: Record<string, string> }>
		| undefined = undefined
	let connectsManual:
		| [string, { img?: string; instructions: string[]; key?: string }][]
		| undefined = undefined
	let args: any = {}
	let renderDescription = true

	$: $workspaceStore && loadResources()

	$: linkedSecretCandidates = apiTokenApps[resourceType]?.linkedSecret
		? ([apiTokenApps[resourceType]?.linkedSecret] as string[])
		: args != undefined
		? Object.keys(args).filter((x) =>
				['token', 'secret', 'key', 'pass', 'private'].some((y) => x.toLowerCase().includes(y))
		  )
		: undefined

	$: linkedSecret =
		forceSecretValue(resourceType) ??
		linkedSecretCandidates?.sort((ua, ub) => linkedSecretValue(ub) - linkedSecretValue(ua))?.[0]

	let scopes: string[] = []
	let extra_params: [string, string][] = []

	let path: string
	let description = ''

	let drawer: Drawer
	let resourceType = ''
	let resourceTypeInfo: ResourceType | undefined = undefined

	let no_back = false

	let pathError = ''

	export async function open(rt?: string) {
		step = 1
		value = ''
		description = ''
		no_back = false
		resourceType = rt ?? ''
		valueToken = undefined
		await loadConnects()

		const connect = connects?.[resourceType]
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
		resourceType = rt
		value = $oauthStore?.access_token!
		valueToken = $oauthStore!
		$oauthStore = undefined
		manual = false
		step = 3
		no_back = true
		drawer.openDrawer?.()
	}

	async function loadConnects() {
		const nconnects = (await OauthService.listOauthConnects()) as any
		if (nconnects['supabase_wizard']) {
			delete nconnects['supabase_wizard']
		}
		connects = nconnects
	}

	const connectAndManual = ['gitlab']

	export async function loadResources() {
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

	export async function next() {
		if (step == 1 && manual) {
			resourceTypeInfo = await ResourceService.getResourceType({
				workspace: $workspaceStore!,
				path: resourceType
			})
			step += 1
			args = {}
		} else if (step == 1 && !manual) {
			const url = new URL(`/api/oauth/connect/${resourceType}`, window.location.origin)
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
							client: resourceType
						}
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
			sendUserToast(`Saved resource${saveVariable ? ' and variable' : ''} path: ${path}`)
			drawer.closeDrawer?.()
		}
	}

	export async function back() {
		if (step > 1) {
			step -= 1
		}
	}

	const dispatch = createEventDispatcher()

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
					color={key === resourceType ? 'blue' : 'light'}
					btnClasses={key === resourceType ? '!border-2' : 'm-[1px]'}
					on:click={() => {
						manual = false
						resourceType = key
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
	{#if manual == false && resourceType != ''}
		<h3>Scopes</h3>
		{#if !manual && resourceType != ''}
			<OauthScopes bind:scopes />
		{/if}
	{/if}

	<h2 class="mt-8 mb-4">Others</h2>

	{#if connectsManual && connectsManual?.length < 10}
		<div class="text-secondary p-2">
			Resource Types have not been synced with the hub. Go to the admins workspace to sync them (and
			add a schedule to do daily):
			<p class="mt-4"
				>1. Go to the "admins" workspaces:
				<img src="/sync_resource_types.png" alt="sync resource types" class="mt-2" />
			</p>
			<p class="mt-4">
				2: Run the synchronization script:
				<img src="/sync_resource_types2.png" alt="sync resource types" class="mt-2" />
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
						btnClasses={key === resourceType ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						on:click={() => {
							manual = true
							resourceType = key
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

	<div class="mt-8 mb-4" />
	<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
		{#if filteredConnectsManual}
			{#each filteredConnectsManual as [key, _]}
				{#if !nativeLanguagesCategory.includes(key)}
					<!-- Exclude specific items -->
					<Button
						size="sm"
						variant="border"
						color={key === resourceType ? 'blue' : 'light'}
						btnClasses={key === resourceType ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						on:click={() => {
							manual = true
							resourceType = key
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
				<img class="m-auto max-h-60" alt="connect" src={apiTokenApps[resourceType].img} />
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
			<textarea use:autosize bind:value={description} placeholder={'Resource description'} />
		</div>
	{:else if description == undefined || description == ''}
		<div class="text-sm text-tertiary">No description provided</div>
	{:else}
		<div class="mt-2" />
		<GfmMarkdown md={description} />
	{/if}
	<div class="mt-12">
		{#key resourceTypeInfo}
			<ApiConnectForm
				{linkedSecret}
				{linkedSecretCandidates}
				{resourceType}
				{resourceTypeInfo}
				bind:args
				bind:isValid
			/>
		{/key}
	</div>
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
