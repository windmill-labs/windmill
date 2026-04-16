<script lang="ts">
	import TableCustom from './TableCustom.svelte'

	import {
		GroupService,
		UserService,
		GranularAclService,
		ResourceService,
		FolderService
	} from '$lib/gen'
	import type { Folder } from '$lib/gen/types.gen'
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isOwner } from '$lib/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import Toggle from './Toggle.svelte'
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
		| 'volume'
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

	let linkedVarPaths: string[] = $state([])
	let alsoApplyToLinked: boolean = $state(true)

	let defaultPerms: { label: string; write: boolean }[] = $state([])
	let defaultPermsLabel: string = $state('')

	async function loadDefaultPerms() {
		const currentPath = path
		const parts = currentPath.split('/')
		if (parts[0] === 'f' && parts.length >= 2) {
			const folderName = parts[1]
			defaultPermsLabel = `Folder f/${folderName} permissions`
			try {
				const folder: Folder = await FolderService.getFolder({
					workspace: $workspaceStore!,
					name: folderName
				})
				if (path !== currentPath) return
				const perms: { label: string; write: boolean }[] = []
				for (const owner of folder.owners) {
					perms.push({ label: owner, write: true })
				}
				for (const [owner, write] of Object.entries(folder.extra_perms ?? {})) {
					if (!folder.owners.includes(owner)) {
						perms.push({ label: owner, write })
					}
				}
				defaultPerms = perms
			} catch {
				if (path !== currentPath) return
				defaultPerms = []
			}
		} else if (parts[0] === 'u' && parts.length >= 2) {
			defaultPermsLabel = `User u/${parts[1]} permissions`
			defaultPerms = [{ label: `u/${parts[1]}`, write: true }]
		} else if (parts[0] === 'g' && parts.length >= 2) {
			defaultPermsLabel = `Group g/${parts[1]} permissions`
			defaultPerms = [{ label: `g/${parts[1]}`, write: true }]
		} else {
			defaultPerms = []
			defaultPermsLabel = ''
		}
	}

	function collectVarRefs(value: unknown): string[] {
		const paths: string[] = []
		function walk(v: unknown) {
			if (typeof v === 'string' && v.startsWith('$var:')) {
				paths.push(v.substring('$var:'.length))
			} else if (Array.isArray(v)) {
				v.forEach(walk)
			} else if (v && typeof v === 'object') {
				Object.values(v).forEach(walk)
			}
		}
		walk(value)
		return [...new Set(paths)]
	}

	async function loadLinkedVarPaths() {
		if (kind !== 'resource') {
			linkedVarPaths = []
			return
		}
		const currentPath = path
		try {
			const resource = await ResourceService.getResource({
				workspace: $workspaceStore!,
				path: currentPath
			})
			if (path !== currentPath) return
			linkedVarPaths = collectVarRefs(resource.value)
		} catch {
			if (path !== currentPath) return
			linkedVarPaths = []
		}
	}

	let own = $state(false)
	export async function openDrawer(newPath: string, kind_l: Kind, isOwnerOverride?: boolean) {
		path = newPath
		kind = kind_l
		alsoApplyToLinked = true
		loadAcls()
		loadGroups()
		loadUsernames()
		loadLinkedVarPaths()
		loadDefaultPerms()
		if (isOwnerOverride !== undefined) {
			own = isOwnerOverride
		} else {
			loadOwner()
		}
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
			if (alsoApplyToLinked) {
				for (const varPath of linkedVarPaths) {
					try {
						await GranularAclService.removeGranularAcls({
							workspace: $workspaceStore!,
							path: varPath,
							kind: 'variable',
							requestBody: { owner }
						})
					} catch (err) {
						sendUserToast(`Failed to update variable ${varPath}: ${err}`, true)
					}
				}
			}
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
		if (alsoApplyToLinked) {
			for (const varPath of linkedVarPaths) {
				try {
					await GranularAclService.addGranularAcls({
						workspace: $workspaceStore!,
						path: varPath,
						kind: 'variable',
						requestBody: { owner, write }
					})
				} catch (err) {
					sendUserToast(`Failed to update variable ${varPath}: ${err}`, true)
				}
			}
		}
		loadAcls()
		dispatch('change', { path, kind })
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Permissions for {path}" on:close={drawer?.closeDrawer}>
		<div class="flex flex-col gap-4">
			{#if defaultPerms.length > 0}
				<div class="flex flex-col gap-1">
					<span class="text-sm font-semibold text-emphasis">{defaultPermsLabel}</span>
					{#each defaultPerms as perm (perm.label)}
						<div class="flex items-center justify-between text-xs text-primary">
							<span>{perm.label}</span>
							<span class="text-tertiary">{perm.write ? 'Writer' : 'Viewer'}</span>
						</div>
					{/each}
				</div>
			{/if}
			<div class="flex flex-col gap-2">
				<span class="text-sm font-semibold text-emphasis"
					>Extra permissions ({acls?.length ?? 0})</span
				>
				{#if linkedVarPaths.length > 0}
					<div class="flex flex-col gap-1.5 p-3 border rounded bg-surface-secondary text-xs">
						<Toggle
							size="xs"
							bind:checked={alsoApplyToLinked}
							options={{ right: 'Also apply to linked variables' }}
						/>
						<ul class="text-2xs text-secondary list-disc ml-4">
							{#each linkedVarPaths as varPath (varPath)}
								<li>{varPath}</li>
							{/each}
						</ul>
					</div>
				{/if}
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
							{#snippet headerRow()}
								<tr>
									<th>owner</th>
									<th></th>
									<th></th>
								</tr>
							{/snippet}
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
		</div></DrawerContent
	>
</Drawer>
