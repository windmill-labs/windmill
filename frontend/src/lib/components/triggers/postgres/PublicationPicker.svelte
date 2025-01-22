<script lang="ts">
	import Select from '$lib/components/apps/svelte-select/lib/Select.svelte'
	import { Button } from '$lib/components/common'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import type { Relations } from '$lib/gen'
	import { PostgresTriggerService } from '$lib/gen/services.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { emptyString } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'

	export let items: string[] = []
	export let publication_name: string = ''
	export let postgres_resource_path: string = ''
	export let table_to_track: Relations[] = []
	export let relations: Relations[] = []
	export let transaction_to_track: string[] = []
	export let selectedTable: 'all' | 'specific' = 'specific'

	async function listDatabasePublication() {
		try {
			const publications = await PostgresTriggerService.listPostgresPublication({
				path: postgres_resource_path,
				workspace: $workspaceStore!
			})

			items = publications
		} catch (error) {
			sendUserToast(error.body, true)
		}
	}

	async function updatePublication() {
		try {
			const message = await PostgresTriggerService.updatePostgresPublication({
				path: postgres_resource_path,
				workspace: $workspaceStore!,
				publication: publication_name,
				requestBody: {
					table_to_track,
					transaction_to_track: transaction_to_track
				}
			})
			sendUserToast(message)
		} catch (error) {
			sendUserToast(error.body, true)
		}
	}

	async function deletePublication() {
		try {
			const message = await PostgresTriggerService.deletePostgresPublication({
				path: postgres_resource_path,
				workspace: $workspaceStore!,
				publication: publication_name
			})
			items = items.filter((item) => item != publication_name)
			table_to_track = []
			publication_name = ''
			sendUserToast(message)
		} catch (error) {
			sendUserToast(error.body, true)
		}
	}

	async function getAllRelations() {
		try {
			const publication_data = await PostgresTriggerService.getPostgresPublication({
				path: postgres_resource_path,
				workspace: $workspaceStore!,
				publication: publication_name
			})
			transaction_to_track = [...publication_data.transaction_to_track]
			relations = publication_data.table_to_track ?? []
			if (relations.length === 0) {
				selectedTable = 'all'
			} else {
				selectedTable = 'specific'
			}
			selectedTable = selectedTable
		} catch (error) {
			sendUserToast(error.body, true)
		}
	}

	listDatabasePublication()

	let darkMode = false

	$: publication_name && getAllRelations()
</script>

<DarkModeObserver bind:darkMode />

<div class="flex gap-1">
	<Select
		class="grow shrink max-w-full"
		bind:justValue={publication_name}
		value={publication_name}
		{items}
		placeholder="Choose a publication"
		inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
		containerStyles={darkMode
			? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
			: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
		portal={false}
		on:select={getAllRelations}
	/>
	<Button
		variant="border"
		color="light"
		wrapperClasses="self-stretch"
		on:click={listDatabasePublication}
		startIcon={{ icon: RefreshCw }}
		iconOnly
	/>
	<Button
		color="light"
		size="xs"
		variant="border"
		disabled={emptyString(publication_name)}
		on:click={updatePublication}>Update</Button
	>
	<Button
		color="light"
		size="xs"
		variant="border"
		disabled={emptyString(publication_name)}
		on:click={deletePublication}>Delete</Button
	>
</div>
