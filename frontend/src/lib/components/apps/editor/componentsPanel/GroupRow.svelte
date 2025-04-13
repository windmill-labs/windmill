<script lang="ts">
	import { deleteGroup, getGroup, updateGroup } from './groupUtils'
	import { workspaceStore } from '$lib/stores'
	import Cell from '$lib/components/table/Cell.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { Trash } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import GroupNameEditor from './NameEditor.svelte'

	export let row: {
		name: string
		path: string
	}

	const dispatch = createEventDispatcher()

	async function toggleDelete() {
		if ($workspaceStore) {
			await deleteGroup($workspaceStore, row.path)
		}
		dispatch('reloadGroups')
		sendUserToast('Group deleted:\n' + row.name)
	}

	function getItems() {
		return [
			{
				action: toggleDelete,
				icon: Trash,
				displayName: 'Delete',
				type: 'delete' as const
			}
		]
	}
</script>

<tr>
	<Cell first>
		<div class="flex flex-row gap-1 items-center">
			<GroupNameEditor
				kind="group"
				on:update={async (e) => {
					if (!$workspaceStore) return
					const group = await getGroup($workspaceStore, row.path)
					await updateGroup($workspaceStore, row.path, {
						value: {
							...group,
							name: e.detail.name
						}
					})
					dispatch('reloadGroups')

					sendUserToast('Group name updated:\n' + e.detail.name)
				}}
				{row}
			/>
			{row.name}
		</div>
	</Cell>

	<Cell last>
		<Dropdown items={getItems()} />
	</Cell>
</tr>
