<script lang="ts">
	import { run } from 'svelte/legacy'

	import TableCustom from './TableCustom.svelte'

	import { GroupService, UserService, GranularAclService } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import AutoComplete from 'simple-svelte-autocomplete'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Tooltip from './Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isOwner } from '$lib/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'

	const dispatch = createEventDispatcher()

	type Kind =
		| 'script'
		| 'group_'
		| 'resource'
		| 'schedule'
		| 'variable'
		| 'flow'
		| 'app'
		| 'raw_app'
		| 'http_trigger'
		| 'websocket_trigger'
		| 'kafka_trigger'
		| 'nats_trigger'
		| 'mqtt_trigger'
		| 'sqs_trigger'
		| 'postgres_trigger'
		| 'gcp_trigger'
	let kind: Kind

	let path: string = $state('')

	let ownerKind: 'user' | 'group' = $state('user')
	let owner: string = $state('')

	let newOwner: string = $state('')
	let write: boolean = false
	let acls: [string, boolean][] = $state([])
	let groups: String[] = $state([])
	let usernames: string[] = $state([])

	let drawer: Drawer | undefined = $state()

	run(() => {
		newOwner = [ownerKind === 'group' ? 'g' : 'u', owner].join('/')
	})

	let own = $state(false)
	export async function openDrawer(newPath: string, kind_l: Kind) {
		path = newPath
		kind = kind_l
		loadAcls()
		loadGroups()
		loadUsernames()
		loadOwner()
		drawer?.openDrawer()
	}

	async function loadOwner() {
		own = isOwner(path, $userStore!, $workspaceStore!)
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
			dispatch('change', { path, kind })
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
		dispatch('change', { path, kind })
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Share {path}" on:close={drawer?.closeDrawer}>
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
								{#snippet children({ item })}
									<ToggleButton value="user" size="xs" label="User" {item} />
									<ToggleButton value="group" size="xs" label="Group" {item} />
								{/snippet}
							</ToggleButtonGroup>
						</div>
						{#key ownerKind}
							<AutoComplete
								required
								noInputStyles
								items={ownerKind === 'user' ? usernames : groups}
								bind:selectedItem={owner}
							/>
						{/key}
						<Button size="sm" on:click={() => addAcl(newOwner, write)}>Add permission</Button>
					</div>
				{/if}
				<TableCustom>
					<!-- @migration-task: migrate this slot by hand, `header-row` is an invalid identifier -->
					<tr slot="header-row">
						<th>owner</th>
						<th></th>
						<th></th>
					</tr>
					{#snippet body()}
						<tbody>
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
													{#snippet children({ item })}
														<ToggleButton value="viewer" size="xs" label="Viewer" {item} />
														<ToggleButton value="writer" size="xs" label="Writer" {item} />
													{/snippet}
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
					{/snippet}
				</TableCustom>
			</div>
		</div>
	</DrawerContent>
</Drawer>
