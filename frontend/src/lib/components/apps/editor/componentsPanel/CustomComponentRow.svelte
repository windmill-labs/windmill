<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Cell from '$lib/components/table/Cell.svelte'

	import { Trash } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import NameEditor from './NameEditor.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { ResourceService } from '$lib/gen'

	export let row: {
		name: string
		path: string
	}

	const dispatch = createEventDispatcher()

	async function toggleDelete() {
		if ($workspaceStore) {
			await ResourceService.deleteResource({
				workspace: $workspaceStore,
				path: row.path
			})
		}
		dispatch('reload')
		sendUserToast('Component deleted:\n' + row.name)
	}

	async function updateName(name: string) {
		if (!$workspaceStore) return
		const cc = await ResourceService.getResourceValue({
			workspace: $workspaceStore ?? '',
			path: row.path
		})
		await ResourceService.updateResource({
			workspace: $workspaceStore ?? '',
			path: row.path,
			requestBody: {
				path: `f/app_custom/${name.replace(/-/g, '_').replace(/\s/g, '_')}`,
				value: {
					...(cc ?? {}),
					name: name
				}
			}
		})
		dispatch('reload')

		sendUserToast('Component name updated:\n' + name)
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
			<NameEditor
				kind="custom component"
				on:update={(e) => {
					updateName(e.detail.name)
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
