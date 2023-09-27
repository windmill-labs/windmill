<script lang="ts">
	import { getGroup, updateGroup } from './groupUtils'
	import { workspaceStore } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'

	import { Popup } from '$lib/components/common'
	import { Pen } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'

	export let row: {
		name: string
		path: string
	}

	let editedName = row.name

	const dispatch = createEventDispatcher()
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
	let:close
>
	<svelte:fragment slot="button">
		<Button color="light" size="xs2" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center">
				<Pen size={16} />
			</div>
		</Button>
	</svelte:fragment>
	<div class="flex flex-col w-80 gap-2">
		<div class="leading-6 font-semibold text-xs">Edit group name</div>
		<div class="flex flex-row gap-2">
			<input bind:value={editedName} />
			<Button
				color="dark"
				size="xs"
				on:click={async () => {
					if (!$workspaceStore) return
					const group = await getGroup($workspaceStore, row.path)
					await updateGroup($workspaceStore, row.path, {
						value: {
							...group,
							name: editedName
						}
					})
					dispatch('reloadGroups')
					close(null)
					sendUserToast('Group name updated:\n' + editedName)
				}}
			>
				Update
			</Button>
		</div>
	</div>
</Popup>
