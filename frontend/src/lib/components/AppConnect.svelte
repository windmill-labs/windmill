<script lang="ts" context="module">
	const apiTokenApps: Record<string, { img?: string; instructions: string }> = {
		airtable: {
			img: 'airtable_connect.png',
			instructions: 'Click on the top-right avatar -> Account -> Api'
		}
	}
</script>

<script lang="ts">
	import IconedResourceType from './IconedResourceType.svelte'
	import PageHeader from './PageHeader.svelte'
	import { workspaceStore, userStore, oauthStore } from '$lib/stores'
	import { faMinus, faPlus } from '@fortawesome/free-solid-svg-icons'

	import { OauthService, ResourceService, VariableService } from '$lib/gen'

	import { createEventDispatcher, onMount } from 'svelte'
	import Modal from './Modal.svelte'
	import Icon from 'svelte-awesome'
	import Path from './Path.svelte'
	import Password from './Password.svelte'
	import { sendUserToast, truncate, truncateRev } from '$lib/utils'
	import { goto } from '$app/navigation'

	let manual = false
	let value = ''
	let connects: Record<string, string[]> = {}
	let connectsManual: [string, { img?: string; instructions: string }][] = []

	let scopes: string[] = []
	let path: string

	let modal: Modal
	let resource_type = ''
	let step = 1

	let no_back = false
	export function open() {
		step = 1
		value = ''
		resource_type = ''
		no_back = false
		modal.openModal()
	}

	export function openFromOauth(rt: string) {
		resource_type = rt
		value = $oauthStore!
		$oauthStore = undefined
		manual = false
		step = 3
		no_back = true
		modal.openModal()
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
			window.location.href = `/api/oauth/connect/${resource_type}?scopes=${scopes.join('+')}`
		} else {
			let exists = true
			try {
				await VariableService.getVariable({
					workspace: $workspaceStore!,
					path
				})
			} catch (e) {
				exists = false
			}
			if (exists) {
				throw Error(`Variable at path ${path} already exists. Delete it or pick another path`)
			}
			exists = true
			try {
				await ResourceService.getResource({
					workspace: $workspaceStore!,
					path
				})
			} catch (e) {
				exists = false
			}
			if (exists) {
				throw Error(`Resource at path ${path} already exists. Delete it or pick another path`)
			}
			await VariableService.createVariable({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					value,
					is_secret: true,
					description: `OAuth token for ${resource_type}`
				}
			})
			await ResourceService.createResource({
				workspace: $workspaceStore!,
				requestBody: {
					resource_type,
					path,
					value: { token: `$var:${path}` },
					description: `OAuth token for ${resource_type}`
				}
			})
			dispatch('refresh')
			sendUserToast(`App token set at resource and variable path: ${path}`)
			modal.closeModal()
		}
	}

	async function back() {
		if (step > 1) {
			step -= 1
		}
	}

	const dispatch = createEventDispatcher()

	$: {
		if ($workspaceStore) {
			loadResources()
		}
	}

	onMount(() => {
		loadConnects()
	})
</script>

<Modal
	bind:this={modal}
	on:close={() => {
		dispatch('close')
	}}
>
	<div slot="title">Connect an app</div>
	<div slot="content">
		{#if step == 1}
			<PageHeader title="Oauth apps" />
			<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
				{#each Object.entries(connects) as [key, values]}
					<button
						class="px-4 h-8 {key == resource_type ? 'item-button-selected' : 'item-button'}"
						on:click={() => {
							manual = false
							resource_type = key
							scopes = values
							dispatch('click')
						}}
					>
						<IconedResourceType name={key} after={true} />
					</button>
				{/each}
			</div>
			<PageHeader title="scopes" primary={false} />
			{#if !manual && resource_type != ''}
				{#each scopes as v}
					<div class="flex flex-row max-w-md">
						<input type="text" bind:value={v} />
						<button
							class="default-button-secondary mx-6"
							on:click={() => {
								scopes = scopes.filter((el) => el != v)
							}}><Icon data={faMinus} class="mb-1" /></button
						>
					</div>
				{/each}
				<button
					class="default-button-secondary mt-1"
					on:click={() => {
						resource_type = resource_type.concat('')
					}}>Add item &nbsp;<Icon data={faPlus} class="mb-1" /></button
				><span class="ml-2">{(resource_type ?? []).length} item(s)</span>
			{:else}
				<p class="italic text-sm">Pick an oauth app and customize the scopes here</p>
			{/if}
			<PageHeader title="API token apps" />
			<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
				{#each connectsManual as [key, instructions]}
					<button
						class="px-4 h-8 {key == resource_type ? 'item-button-selected' : 'item-button'}"
						on:click={() => {
							manual = true
							resource_type = key
							dispatch('click')
						}}
					>
						<IconedResourceType name={key} after={true} />
					</button>
				{/each}
			</div>
		{:else if step == 2}
			{#if manual}
				<PageHeader title="Instructions" />
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
			<Path bind:path initialPath={`u/${$userStore?.username ?? ''}/my_${resource_type}`} />
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
					tokens it accepts to this app.
				</li>
			</ul>
		{/if}
	</div>
	<div slot="submission">
		{#if step > 1 && !no_back}
			<button class="default-button px-4 py-2 font-semibold" on:click={back}>Back</button>
		{/if}
		<button
			class="default-button px-4 py-2 font-semibold"
			class:default-button-disabled={(step == 1 && resource_type == '') ||
				(step == 2 && value == '')}
			on:click={next}
		>
			{step == 3 ? 'Connect' : 'Next'}
		</button>
	</div>
</Modal>

<style>
	.item-button {
		@apply py-1;
		@apply border;
		@apply rounded-sm;
	}
	.item-button-selected {
		@apply py-1;
		@apply border border-blue-500;
		@apply bg-blue-50;
		@apply rounded-sm;
	}

	.selected:hover {
		@apply border border-gray-400 rounded-md border-opacity-50;
	}
</style>
