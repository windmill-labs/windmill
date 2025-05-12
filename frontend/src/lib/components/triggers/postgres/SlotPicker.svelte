<script lang="ts">
	import Select from '$lib/components/apps/svelte-select/lib/Select.svelte'
	import { Button } from '$lib/components/common'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import { PostgresTriggerService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { emptyString } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'

	export let edit: boolean
	export let replication_slot_name: string = ''
	export let postgres_resource_path: string = ''

	let deletingSlot: boolean = false
	let loadingSlot: boolean = false
	let items: (string | undefined)[] = []
	async function listDatabaseSlot() {
		try {
			loadingSlot = true
			const result = await PostgresTriggerService.listPostgresReplicationSlot({
				path: postgres_resource_path,
				workspace: $workspaceStore!
			})

			let exist = false

			items = result
				.map((slot) => {
					if (replication_slot_name && replication_slot_name === slot.slot_name) {
						exist = true
					}
					if (!emptyString(slot.slot_name) && !slot.active) {
						return slot.slot_name
					}
				})
				.filter((slot) => slot != undefined)

			if (edit && replication_slot_name) {
				if (!exist) {
					sendUserToast(`Replication ${replication_slot_name} does not exist`, true)
				}
			}
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			loadingSlot = false
		}
	}

	async function deleteSlot() {
		try {
			deletingSlot = true
			const message = await PostgresTriggerService.deletePostgresReplicationSlot({
				path: postgres_resource_path,
				workspace: $workspaceStore!,
				requestBody: {
					name: replication_slot_name
				}
			})
			items = items.filter((item) => item != replication_slot_name)
			replication_slot_name = ''
			sendUserToast(message)
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			deletingSlot = false
		}
	}

	listDatabaseSlot()

	let darkMode = false
</script>

<DarkModeObserver bind:darkMode />

<div class="flex gap-1">
	<Select
		class="grow shrink max-w-full"
		on:select={(e) => {
			replication_slot_name = e.detail.value
		}}
		on:clear={() => {
			replication_slot_name = ''
		}}
		loading={loadingSlot}
		value={replication_slot_name}
		{items}
		placeholder="Choose a slot name"
		inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
		containerStyles={darkMode
			? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
			: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
		portal={false}
	/>
	<Button
		variant="border"
		color="light"
		wrapperClasses="self-stretch"
		on:click={listDatabaseSlot}
		startIcon={{ icon: RefreshCw }}
		iconOnly
	/>
	<Button
		loading={deletingSlot}
		color="light"
		size="xs"
		variant="border"
		disabled={emptyString(replication_slot_name)}
		on:click={deleteSlot}>Delete</Button
	>
</div>
