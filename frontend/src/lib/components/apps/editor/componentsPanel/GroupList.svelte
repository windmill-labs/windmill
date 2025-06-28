<script lang="ts">
	import { listGroups, type Group, createGroup } from './groupUtils'
	import { workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	import { sendUserToast } from '$lib/toast'
	import { ResourceService } from '$lib/gen'
	import GroupRow from './GroupRow.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import { getAllSubgridsAndComponentIds } from '../appUtils'

	export let item: GridItem
	const { app } = getContext<AppViewerContext>('AppViewerContext')

	let groups: Array<{
		name: string
		path: string
	}> = []

	let loading: boolean = false

	async function getGroups() {
		loading = true
		groups = await listGroups($workspaceStore!)
		loading = false
	}

	function getSubgrids(item: GridItem) {
		let allSubgrids = {}
		let subgrids = getAllSubgridsAndComponentIds($app, item.data)[0]
		for (let key of subgrids) {
			allSubgrids[key] = $app.subgrids?.[key]
		}
		return allSubgrids
	}
	async function addGroup(nameField: string) {
		const groups = await ResourceService.listResourceNames({
			workspace: $workspaceStore!,
			name: 'app_group'
		})

		let subgrids = getSubgrids(item)

		let i = groups.length
		let prefix = `f/app_groups/group_`
		while (true) {
			if (groups.find((g) => g.path.startsWith(prefix + i))) {
				i++
			} else {
				break
			}
		}

		const group: Group = {
			path: prefix + i,
			value: {
				value: { item, subgrids },
				name: nameField
			}
		}

		try {
			const message = await createGroup($workspaceStore!, group)

			sendUserToast('Group created: ' + message)
		} catch (e) {
			sendUserToast(
				'Group creation failed. You need write privilege on folder app_groups: ' + (e.body ?? e),
				true
			)
		}
		getGroups()

		nameField = ''
	}

	let nameField: string = ''

	getGroups()
</script>

<div id="group_portal"></div>

<div class="p-2 flex flex-col items-start w-auto gap-2 relative">
	<div class="w-full flex flex-row gap-2 items-center">
		<input bind:value={nameField} placeholder={'Group name'} />
		<Button on:click={() => addGroup(nameField)} color="dark" size="xs">Create group</Button>
	</div>

	{#if loading}
		<div class="flex flex-col w-full pt-12">
			{#each new Array(6) as _}
				<Skeleton layout={[[2], 0.5]} />
			{/each}
		</div>
	{:else if Array.isArray(groups) && groups.length > 0}
		<div class="w-full">
			<DataTable size="xs">
				<Head>
					<tr>
						<Cell first head>Path</Cell>
						<Cell last head />
					</tr>
				</Head>
				<tbody class="divide-y">
					{#if groups && groups.length > 0}
						{#each groups as row}
							{#key row}
								<GroupRow
									{row}
									on:reloadGroups={() => {
										getGroups()
									}}
								/>
							{/key}
						{/each}
					{:else}
						<tr><td>Loading...</td></tr>
					{/if}
				</tbody>
			</DataTable>
		</div>
	{/if}
</div>
