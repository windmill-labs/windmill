<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Cell from '$lib/components/table/Cell.svelte'

	import { Trash } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'
	import NameEditor from './NameEditor.svelte'

	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { ResourceService } from '$lib/gen/services/ResourceService'

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
</script>

<tr>
	<Cell first>
		<div class="flex flex-row gap-1 items-center">
			<NameEditor
				kind="custom component"
				on:update={async (e) => {
					if (!$workspaceStore) return
					const cc = await ResourceService.getResourceValue({
						workspace: $workspaceStore ?? '',
						path: row.path
					})
					await ResourceService.updateResource({
						workspace: $workspaceStore ?? '',
						path: row.path,
						requestBody: {
							path: `f/app_custom/${e.detail.name.replace(/-/g, '_').replace(/\s/g, '_')}`,
							value: {
								...cc,
								name: e.detail.name
							}
						}
					})
					dispatch('reload')

					sendUserToast('Component name updated:\n' + e.detail.name)
				}}
				{row}
			/>
			{row.name}
		</div>
	</Cell>

	<Cell last>
		<div class={twMerge('flex flex-row gap-1 justify-end  z-[10000]')}>
			<button on:pointerdown|stopPropagation>
				<ButtonDropdown target="#cc_portal" hasPadding={false}>
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
