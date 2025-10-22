<script lang="ts">
	import TableCustom from './TableCustom.svelte'

	import { GroupService, UserService, GranularAclService } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Tooltip from './Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isOwner } from '$lib/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import { Trash } from 'lucide-svelte'

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
		| 'email_trigger'
	let kind: Kind

	let path: string = $state('')

	let ownerKind: 'user' | 'group' = $state('user')
	let owner: string = $state('')

	let newOwner: string = $derived.by(
		() => owner && [ownerKind === 'group' ? 'g' : 'u', owner].join('/')
	)
	let write: boolean = false
	let acls: [string, boolean][] = $state([])
	let groups: String[] = $state([])
	let usernames: string[] = $state([])

	let drawer: Drawer | undefined = $state()

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
		<div class="flex flex-col gap-2">
			<h1 class="text-sm font-semibold text-emphasis">{path}</h1>
			<span class="text-xs font-semibold text-emphasis"
				>Extra Permissions ({acls?.length ?? 0}) &nbsp; <Tooltip
					>Items already have default permissions. If belonging to an user or group, that group or
					user owns it and can write to it as well as modify its permisions and move it. Folders
					have read/write that apply to the whole folder and are additive to the items permissions.</Tooltip
				></span
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
									<ToggleButton value="user" label="User" {item} />
									<ToggleButton value="group" label="Group" {item} />
								{/snippet}
							</ToggleButtonGroup>
						</div>
						{#key ownerKind}
							<Select
								items={safeSelectItems(
									(ownerKind === 'user' ? usernames : groups).map((x) => x.toString())
								)}
								bind:value={owner}
								class="grow min-w-48"
							/>
						{/key}
						<Button
							size="lg"
							variant="accent"
							disabled={!newOwner}
							on:click={() => addAcl(newOwner, write)}>Add permission</Button
						>
					</div>
				{/if}
				{#if acls?.length > 0}
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
															<ToggleButton value="viewer" small label="Viewer" {item} />
															<ToggleButton value="writer" small label="Writer" {item} />
														{/snippet}
													</ToggleButtonGroup>
												</div>
											{:else}{write}{/if}</td
										>
										<td>
											{#if own}
												<Button
													variant="default"
													destructive
													size="xs"
													on:click={() => deleteAcl(owner)}
													startIcon={{ icon: Trash }}
												/>
											{/if}
										</td>
									</tr>
								{/each}
							</tbody>
						{/snippet}
					</TableCustom>
				{/if}
			</div>
		</div>
	</DrawerContent>
</Drawer>
