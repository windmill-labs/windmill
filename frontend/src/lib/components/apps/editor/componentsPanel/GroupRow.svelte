<script lang="ts">
	import { deleteGroup } from './groupUtils'
	import { workspaceStore } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	import { Eye, Trash } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'
	import GroupNameEditor from './GroupNameEditor.svelte'

	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'

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
</script>

<tr>
	<Cell first>
		<div class="flex flex-row gap-1 items-center">
			<GroupNameEditor on:reloadGroups {row} />
			{row.name}
		</div>
	</Cell>

	<Cell last>
		<div class={twMerge('flex flex-row gap-1 justify-end ')}>
			<Button color="light" size="xs" on:click={() => {}}>
				<div class="flex flex-row gap-1 items-center">
					<Eye size={16} />
					Preview
				</div>
			</Button>

			<button on:pointerdown|stopPropagation>
				<ButtonDropdown hasPadding={false}>
					<svelte:fragment slot="items">
						<MenuItem on:click={toggleDelete}>
							<div
								class={classNames(
									'!text-red-600 flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
								)}
							>
								<Trash size={16} />
								Delete
							</div>
						</MenuItem>
					</svelte:fragment>
				</ButtonDropdown>
			</button>
		</div>
	</Cell>
</tr>
