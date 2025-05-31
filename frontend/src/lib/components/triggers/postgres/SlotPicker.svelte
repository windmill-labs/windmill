<script lang="ts">
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/Select.svelte'
	import { PostgresTriggerService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { emptyString } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'

	export let edit: boolean
	export let replication_slot_name: string = ''
	export let postgres_resource_path: string = ''
	export let disabled: boolean = false
	let items: (string | undefined)[] = []
	async function listDatabaseSlot() {
		try {
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
		}
	}

	async function deleteSlot() {
		try {
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
		}
	}

	listDatabaseSlot()
</script>

<div class="flex gap-1">
	<Select
		class="grow shrink max-w-full"
		bind:value={replication_slot_name}
		onClear={() => (replication_slot_name = '')}
		items={items.map((value) => ({ value }))}
		placeholder="Choose a slot name"
		disablePortal
		clearable
		{disabled}
	/>
	<Button
		variant="border"
		color="light"
		wrapperClasses="self-stretch"
		on:click={listDatabaseSlot}
		startIcon={{ icon: RefreshCw }}
		iconOnly
		{disabled}
	/>
	<Button
		color="light"
		size="xs"
		variant="border"
		disabled={emptyString(replication_slot_name) || disabled}
		on:click={deleteSlot}>Delete</Button
	>
</div>
