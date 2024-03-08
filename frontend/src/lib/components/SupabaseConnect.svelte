<script lang="ts">
	import { Loader2, RotateCwIcon } from 'lucide-svelte'

	import { Button, DrawerContent } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import Path from './Path.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'

	import autosize from 'svelte-autosize'
	import { ResourceService, VariableService } from '$lib/gen'
	import { oauthStore, workspaceStore } from '$lib/stores'
	import Password from './Password.svelte'
	import { createEventDispatcher } from 'svelte'

	let drawer: Drawer
	let token: undefined | string = undefined
	export async function open() {
		token = $oauthStore?.access_token ?? ''
		drawer.openDrawer?.()
		step = 'init'
		description = ''
	}

	let step: 'init' | 'resource' = 'init'

	async function listDatabases() {
		if (!token) return
		databases = undefined
		const res = await fetch('/api/oauth/list_supabase', {
			headers: {
				'Content-Type': 'application/json',
				'X-Supabase-Token': token
			}
		})
		databases = await res.json()
	}

	type Database = {
		name: string
		database?: { host: string }
		region: string
		id: string
	}
	let databases: undefined | Database[] = undefined

	$: token != undefined && listDatabases()

	let selectedDatabase: undefined | Database = undefined

	let description = ''
	let pathError = ''
	let password = ''
	let path: string | undefined = undefined

	/**
	 * https://github.com/orgs/supabase/discussions/17817
	 * host is in the format of `aws-0-${region}.pooler.supabase.com`
	 * user is in the format of `postgres.${id}`
	 */
	$: resourceValue = {
		host: `aws-0-${selectedDatabase?.region}.pooler.supabase.com`,
		user: `postgres.${selectedDatabase?.id}`,
		port: 5432,
		dbname: 'postgres',
		sslmode: 'prefer',
		password: `$var:${path}`
	}
	$: disabled = path == undefined || pathError != '' || path == ''

	const dispatch = createEventDispatcher()
	async function save() {
		if (!path) return
		await VariableService.createVariable({
			workspace: $workspaceStore!,
			requestBody: {
				path,
				value: password,
				is_secret: true,
				description: 'Password for supabase postgres database',
				is_oauth: false
			}
		})

		await ResourceService.createResource({
			workspace: $workspaceStore!,
			requestBody: {
				resource_type: 'postgresql',
				path,
				value: resourceValue,
				description
			}
		})
		sendUserToast('Saved postgres resource')
		dispatch('refresh')
		drawer.closeDrawer?.()
	}
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Add a Supabase Database" on:close={drawer.closeDrawer}>
		{#if step === 'init' || selectedDatabase == undefined}
			<h2
				>Connect an existing database <div class="inline-block ml-2"
					><Button
						variant="border"
						color="light"
						wrapperClasses="self-stretch"
						on:click={listDatabases}><RotateCwIcon size={12} /></Button
					></div
				>
			</h2>
			<div class="mt-6" />

			{#if databases == undefined}
				<Loader2 class="animate-spin" />
			{:else}
				<div class=" flex flex-col gap-y-2" />
				{#each databases as database}
					<button
						class="btn btn-outline-primary mt-2 border p-2 w-full border-secondary-inverse hover:border-secondary rounded"
						on:click={() => {
							selectedDatabase = database

							step = 'resource'
						}}
					>
						<div class="flex flex-row items-center">
							<div class="flex-grow">
								<h3 class="text-lg font-semibold">{database.name}</h3>

								<p class="text-sm text-secondary">id: {database.id} - region: {database.region}</p>
							</div>
						</div>
					</button>
				{/each}
			{/if}

			<h3 class="mt-8 mb-2">Create a new database</h3>
			<p class="text-sm text-secondary"
				><a href="https://supabase.com/dashboard/projects" target="_blank" rel="noopener noreferrer"
					>Create a new database in your Supabase account
				</a>
			</p>
		{:else if step === 'resource'}
			<Path
				bind:error={pathError}
				bind:path
				initialPath=""
				fullNamePlaceholder={'supabase_' +
					selectedDatabase?.name?.replace(/\s+/g, '').replace(/[^\w\s]/gi, '')}
				kind="resource"
			/>

			<h2 class="mt-8 mb-2">Database Password</h2>
			<p class="text-sm text-secondary mb-1"
				>For security reasons from supabase, the password of the database cannot be retrieved
				automatically. In a future update, a dedicated role for windmill will be created and the
				password for it will be generated automatically. The password of the database is shown
				during the project creation.</p
			>
			<Password required bind:password />

			<h3 class="mt-6 mb-2">Description</h3>
			<textarea type="text" autocomplete="off" use:autosize bind:value={description} />

			<div class="mt-12" />
			<p class="my-1 text-sm text-secondary"
				>A resource and a variable will be created at path: {path}. The content of the resource will
				be:</p
			>
			<Highlight language={json} code={JSON.stringify(resourceValue, null, 4)} />
		{/if}
		<div slot="actions" class="flex gap-1">
			{#if step == 'resource' && selectedDatabase != undefined}
				<Button variant="border" on:click={() => (step = 'init')}>Back</Button>

				<Button {disabled} on:click={save}>Save</Button>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
