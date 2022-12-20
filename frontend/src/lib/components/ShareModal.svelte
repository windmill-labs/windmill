<script lang="ts">
	import TableCustom from './TableCustom.svelte'

	import { GranularAclService } from '$lib/gen/services/GranularAclService'
	import { isOwner, sendUserToast } from '$lib/utils'
	import { GroupService, UserService } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import AutoComplete from 'simple-svelte-autocomplete'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer, ToggleButton, ToggleButtonGroup } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Tooltip from './Tooltip.svelte'

	const dispatch = createEventDispatcher()

	export let kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow'
	export let path: string = ''

	let ownerKind: 'user' | 'group' = 'user'
	let owner: string = ''

	let newOwner: string = ''
	let write: boolean = false
	let acls: [string, boolean][] = []
	let groups: String[] = []
	let usernames: string[] = []

	let drawer: Drawer

	$: newOwner = [ownerKind === 'group' ? 'g' : 'u', owner].join('/')

	let own = false
	export async function openDrawer(newPath: string) {
		path = newPath
		loadAcls()
		loadGroups()
		loadUsernames()
		drawer.openDrawer()
	}

	$: $userStore && $workspaceStore && loadOwner()

	async function loadOwner() {
		own = await isOwner(path, $userStore!, $workspaceStore!)
	}
	async function loadAcls() {
		acls = Object.entries(
			await GranularAclService.getGranularAcls({ workspace: $workspaceStore!, path, kind })
		)
	}

	async function loadGroups(): Promise<void> {
		groups = await GroupService.listGroupNames({ workspace: $workspaceStore! })
	}

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
	}

	async function deleteAcl(owner: string) {
		try {
			await GranularAclService.removeGranularAcls({
				workspace: $workspaceStore!,
				path,
				kind,
				requestBody: { owner }
			})
			loadAcls()
			dispatch('change')
		} catch (err) {
			sendUserToast(err.toString(), true)
		}
	}

	async function addAcl(owner: string, write: boolean) {
		await GranularAclService.addGranularAcls({
			workspace: $workspaceStore!,
			path,
			kind,
			requestBody: { owner, write }
		})
		loadAcls()
		dispatch('change')
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Share {path}" on:close={drawer.closeDrawer}>
		<div class="flex flex-col gap-6">
			<h1>{path}</h1>
			<h2
				>Extra Permissions ({acls?.length ?? 0}) &nbsp; <Tooltip
					>Items already have default permissions. If belonging to an user or group, that group or
					user owns it and can write to it as well as modify its permisions and move it. Folders
					have read/write that apply to the whole folder and are additive to the items permissions.</Tooltip
				></h2
			>
			{#if !own}
				<Alert type="warning" title="Not owner"
					>Since you do not own this item, you cannot modify its permission</Alert
				>
			{/if}
			<div>
				{#if own}
					<div class="flex flex-row flex-wrap gap-2 items-center">
						<div>
							<ToggleButtonGroup bind:selected={ownerKind} on:selected={() => (owner = '')}>
								<ToggleButton position="left" value="user" size="xs">User</ToggleButton>
								<ToggleButton position="right" value="group" size="xs">Group</ToggleButton>
							</ToggleButtonGroup>
						</div>
						{#key ownerKind}
							<AutoComplete
								items={ownerKind === 'user' ? usernames : groups}
								bind:selectedItem={owner}
							/>
						{/key}
						<Button size="sm" on:click={() => addAcl(newOwner, write)}>Add permission</Button>
					</div>
				{/if}
				<TableCustom>
					<tr slot="header-row">
						<th>owner</th>
						<th />
						<th />
					</tr>
					<tbody slot="body">
						{#each acls as [owner, write]}
							<tr>
								<td>{owner}</td>
								<td
									>{#if own}
										<div>
											<ToggleButtonGroup
												selected={write ? 'writer' : 'viewer'}
												on:selected={async (e) => {
													const role = e.detail
													if (role == 'writer') {
														await addAcl(owner, true)
													} else {
														await addAcl(owner, false)
													}
													loadAcls()
												}}
											>
												<ToggleButton position="left" value="viewer" size="xs">Viewer</ToggleButton>
												<ToggleButton position="right" value="writer" size="xs">Writer</ToggleButton
												>
											</ToggleButtonGroup>
										</div>
									{:else}{write}{/if}</td
								>
								<td>
									{#if own}
										<Button
											variant="border"
											color="red"
											size="xs"
											on:click={() => deleteAcl(owner)}
										>
											Delete
										</Button>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</TableCustom>
			</div>
		</div>
	</DrawerContent>
</Drawer>
