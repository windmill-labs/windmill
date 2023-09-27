<script lang="ts">
	import { listGroups, type Group, createGroup } from './groupUtils'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	import { sendUserToast } from '$lib/toast'
	import { ResourceService } from '$lib/gen'
	import { Alert } from '$lib/components/common'
	import GroupRow from './GroupRow.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'

	export let selectedGroup: object = {}

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

	async function addGroup(nameField: string) {
		const groups = await ResourceService.listResourceNames({
			workspace: $workspaceStore!,
			name: 'app_themes'
		})

		const group: Group = {
			path: 'f/app_themes/group_' + groups.length,
			value: {
				value: selectedGroup,
				name: nameField
			}
		}

		const message = await createGroup($workspaceStore!, group)

		getGroups()

		nameField = ''

		sendUserToast('Group created:' + message)
	}

	let nameField: string = ''

	onMount(() => {
		getGroups()
	})
</script>

<div class="p-2 flex flex-col items-start w-auto gap-2 relative">
	{#if $enterpriseLicense === undefined}
		<div class="absolute top-0 left-0 w-full h-full bg-gray-50 opacity-50 z-10 bottom-0" />
		<Alert
			type="warning"
			title="Groups are available in the enterprise edition."
			class="w-full z-50"
			size="xs"
		>
			Upgrade to the enterprise edition to use groups.
		</Alert>
	{/if}
	<div class="w-full flex flex-row gap-2 items-center">
		<input bind:value={nameField} placeholder={'TODO'} />
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
						<tr>Loading...</tr>
					{/if}
				</tbody>
			</DataTable>
		</div>
	{/if}
</div>
