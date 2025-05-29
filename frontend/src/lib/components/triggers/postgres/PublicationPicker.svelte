<script lang="ts">
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/Select.svelte'
	import type { Relations } from '$lib/gen'
	import { PostgresTriggerService } from '$lib/gen/services.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { emptyString } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'

	export let items: string[] = []
	export let can_write: boolean = true
	export let publication_name: string = ''
	export let postgres_resource_path: string = ''
	export let relations: Relations[] | undefined = undefined
	export let transaction_to_track: string[] = []
	export let disabled: boolean = false

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
					table_to_track: relations,
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
			relations = undefined
			transaction_to_track = ['Insert', 'Update', 'Delete']
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
			relations =
				publication_data.table_to_track && publication_data.table_to_track.length > 0
					? publication_data.table_to_track
					: undefined
		} catch (error) {
			sendUserToast(error.body, true)
		}
	}

	listDatabasePublication()
	$: publication_name && getAllRelations()
</script>

<div class="flex gap-1">
	<Select
		disabled={!can_write || disabled}
		class="grow shrink"
		bind:value={publication_name}
		items={items.map((value) => ({ value }))}
		placeholder="Choose a publication"
		clearable
		disablePortal
	/>
	<Button
		disabled={!can_write || disabled}
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
		disabled={emptyString(publication_name) || !can_write || disabled}
		on:click={updatePublication}>Update</Button
	>
	<Button
		color="light"
		size="xs"
		variant="border"
		disabled={emptyString(publication_name) || !can_write || disabled}
		on:click={deletePublication}>Delete</Button
	>
</div>
