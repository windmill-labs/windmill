<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		type Folder,
		type FolderDefaultPermissionedAs,
		FolderService,
		UserService,
		GranularAclService,
		GroupService
	} from '$lib/gen'
	import TableCustom from './TableCustom.svelte'
	import { Alert, Button, Drawer, DrawerContent } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import GroupEditor from './GroupEditor.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { ArrowDown, ArrowUp, Eye, Plus, Trash } from 'lucide-svelte'
	import Label from './Label.svelte'
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher, untrack } from 'svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import PermissionHistory from './PermissionHistory.svelte'
	import { Minimatch } from 'minimatch'
	import Tooltip from './Tooltip.svelte'
	import CollapseLink from './CollapseLink.svelte'

	interface Props {
		name: string
	}

	let { name }: Props = $props()
	let can_write = $state(false)

	type Role = 'viewer' | 'writer' | 'admin'
	let folder: Folder | undefined
	let perms: { owner_name: string; role: Role }[] | undefined = $state(undefined)
	let usernames: string[] = $state([])
	let groups: string[] = $state([])
	let ownerItem: string = $state('')

	let newGroup: Drawer | undefined = $state(undefined)
	let viewGroup: Drawer | undefined = $state(undefined)

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
	}

	async function loadGroups(): Promise<void> {
		groups = await GroupService.listGroupNames({ workspace: $workspaceStore! })
	}

	async function load() {
		loadUsernames()
		loadGroups()
		await loadFolder()
	}

	async function addToFolder() {
		await GranularAclService.addGranularAcls({
			workspace: $workspaceStore ?? '',
			path: name,
			kind: 'folder',
			requestBody: {
				owner: (ownerKind == 'user' ? 'u/' : 'g/') + ownerItem
			}
		})
		ownerItem = ''
		loadFolder()
	}

	let folderNotFound: boolean | undefined = $state(undefined)

	async function loadFolder(): Promise<void> {
		try {
			folder = await FolderService.getFolder({ workspace: $workspaceStore!, name })
			summary = folder.summary ?? ''
			defaultPermissionedAs = (folder.default_permissioned_as ?? []).map((r) => ({ ...r }))
			can_write =
				$userStore != undefined &&
				(folder?.owners.includes('u/' + $userStore.username) ||
					($userStore.is_admin ?? false) ||
					($userStore.is_super_admin ?? false) ||
					$userStore.pgroups.findIndex((x) => folder?.owners.includes(x)) != -1)

			perms = Array.from(
				new Set(
					Object.entries(folder?.extra_perms ?? {})
						.map((x) => x[0])
						.concat(folder?.owners ?? [])
				)
			).map((x) => {
				return {
					owner_name: x,
					role: getRole(x)
				}
			})
			reloadHistory++
		} catch (e) {
			folderNotFound = true
		}
	}

	// --- default_permissioned_as rules editor ---
	let defaultPermissionedAs: FolderDefaultPermissionedAs = $state([])

	const canEditDefaults = $derived(
		can_write &&
			($userStore?.is_admin ||
				$userStore?.is_super_admin ||
				($userStore?.groups ?? []).includes('wm_deployers'))
	)

	function isValidGlob(glob: string): boolean {
		if (!glob) return false
		try {
			new Minimatch(glob)
			return true
		} catch {
			return false
		}
	}

	function isValidPermissionedAs(value: string): boolean {
		return /^[ug]\/.+/.test(value) || value.includes('@')
	}

	// Split a permissioned_as value like "u/alice" or "g/prod" into its kind and name.
	function ruleKind(value: string): 'user' | 'group' {
		return value.startsWith('g/') ? 'group' : 'user'
	}
	function ruleName(value: string): string {
		if (value.startsWith('u/') || value.startsWith('g/')) return value.slice(2)
		return value
	}
	function setRulePermissionedAs(idx: number, kind: 'user' | 'group', name: string) {
		const prefix = kind === 'user' ? 'u/' : 'g/'
		defaultPermissionedAs[idx].permissioned_as = prefix + name
	}

	const defaultRulesInvalid = $derived(
		defaultPermissionedAs.some(
			(r) => !isValidGlob(r.path_glob) || !isValidPermissionedAs(r.permissioned_as)
		)
	)

	function addDefaultRule() {
		defaultPermissionedAs = [...defaultPermissionedAs, { path_glob: '**', permissioned_as: '' }]
	}

	function removeDefaultRule(idx: number) {
		defaultPermissionedAs = defaultPermissionedAs.filter((_, i) => i !== idx)
	}

	function moveDefaultRule(idx: number, delta: -1 | 1) {
		const next = [...defaultPermissionedAs]
		const target = idx + delta
		if (target < 0 || target >= next.length) return
		;[next[idx], next[target]] = [next[target], next[idx]]
		defaultPermissionedAs = next
	}

	async function saveDefaultRules() {
		if (defaultRulesInvalid) {
			sendUserToast('Some rules have invalid globs or permissioned_as values', true)
			return
		}
		try {
			await FolderService.updateFolder({
				workspace: $workspaceStore ?? '',
				name,
				requestBody: { default_permissioned_as: defaultPermissionedAs }
			})
			sendUserToast('Default permissioned_as rules updated')
			dispatch('update')
			loadFolder()
		} catch (e) {
			sendUserToast(e.body ?? String(e), true)
		}
	}

	function getRole(x: string): Role {
		const viewer = x in (folder?.extra_perms ?? {})
		const writer = viewer && (folder?.extra_perms ?? {})[x]
		const owner = folder?.owners?.includes(x)

		if (owner) {
			return 'admin'
		} else if (writer) {
			return 'writer'
		} else {
			return 'viewer'
		}
	}

	let ownerKind: 'user' | 'group' = $state('user')
	let groupCreated: string | undefined = $state(undefined)
	let newGroupName: string = $state('')
	let summary: string = $state('')

	async function addGroup() {
		await GroupService.createGroup({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newGroupName }
		})
		groupCreated = newGroupName
		$userStore?.folders?.push(newGroupName)
		loadGroups()
		ownerItem = newGroupName
	}

	const dispatch = createEventDispatcher()

	async function updateFolder() {
		await FolderService.updateFolder({
			workspace: $workspaceStore ?? '',
			name,
			requestBody: { summary }
		})
		sendUserToast('Folder summary updated')
		dispatch('update')
		loadFolder()
	}
	$effect.pre(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				load()
			})
		}
	})

	let reloadHistory = $state(0)
</script>

<Drawer bind:this={newGroup}>
	<DrawerContent
		title="New Group"
		on:close={() => {
			newGroup?.closeDrawer()
			groupCreated = undefined
		}}
	>
		{#if !groupCreated}
			<div class="flex flex-row">
				<input class="mr-2" placeholder="New group name" bind:value={newGroupName} />
				<Button size="md" startIcon={{ icon: Plus }} disabled={!newGroupName} on:click={addGroup}>
					New&nbsp;group
				</Button>
			</div>
		{:else}
			<GroupEditor name={groupCreated} />
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={viewGroup}>
	<DrawerContent title="Group {ownerItem}" on:close={viewGroup.closeDrawer}>
		<GroupEditor name={ownerItem} />
	</DrawerContent>
</Drawer>

<div class="flex flex-col gap-6">
	<Label label="Summary">
		<div class="flex flex-row gap-2">
			<TextInput
				inputProps={{ placeholder: 'Short summary to be displayed when listed' }}
				bind:value={summary}
				size="md"
			/>
			<Button variant="accent" unifiedSize="md" on:click={updateFolder} disabled={!can_write}
				>Save</Button
			>
		</div>
	</Label>

	<Label label={`Permissions (${perms?.length ?? 0})`}>
		<div class="flex flex-col gap-2">
			{#if can_write}
				<Alert type="info" title="New permissions may take up to 60s to apply">
					Due to permissions cache invalidation
				</Alert>
				<div class="flex items-center gap-1">
					<div>
						<ToggleButtonGroup bind:selected={ownerKind} on:selected={() => (ownerItem = '')}>
							{#snippet children({ item })}
								<ToggleButton value="user" label="User" {item} />
								<ToggleButton value="group" label="Group" {item} />
							{/snippet}
						</ToggleButtonGroup>
					</div>

					{#key ownerKind}
						{@const items =
							ownerKind === 'user'
								? usernames.filter((x) => !perms?.map((y) => y.owner_name).includes('u/' + x))
								: groups.filter((x) => !perms?.map((y) => y.owner_name).includes('g/' + x))}
						<Select items={safeSelectItems(items)} bind:value={ownerItem} class="grow min-w-24" />
						{#if ownerKind == 'group'}
							<Button
								title="View Group"
								variant="default"
								unifiedSize="md"
								disabled={!ownerItem || ownerItem == ''}
								on:click={viewGroup.openDrawer}
								startIcon={{ icon: Eye }}
								iconOnly
							/>
							<Button
								title="New Group"
								variant="default"
								on:click={newGroup.openDrawer}
								unifiedSize="md"
								startIcon={{ icon: Plus }}
								iconOnly
							/>
						{/if}
					{/key}
					<Button
						disabled={ownerItem == ''}
						variant="accent"
						unifiedSize="md"
						on:click={addToFolder}
					>
						Grant
					</Button>
				</div>
			{/if}

			{#if folderNotFound}
				<Alert type="warning" title="Folder not found" size="xs">
					The folder "{name}" does not exist in the workspace. You can create it by clicking the
					button below. An item can seemingly be in a folder given its path without the folder
					existing. A windmill folder has settable permissions that its children inherit. If an item
					is within a non-existing folders, only admins will see it.
				</Alert>
				<Button
					variant="default"
					wrapperClasses="w-min"
					startIcon={{ icon: Plus }}
					size="xs"
					on:click={() => {
						FolderService.createFolder({
							workspace: $workspaceStore ?? '',
							requestBody: { name }
						}).then(() => {
							loadFolder()
						})
					}}
				>
					Create folder "{name}"
				</Button>
			{/if}
			{#if perms}
				<TableCustom>
					{#snippet headerRow()}
						<tr>
							<th>user/group</th>
							<th></th>
							<th></th>
						</tr>
					{/snippet}
					{#snippet body()}
						<tbody>
							{#each perms ?? [] as { owner_name, role }}<tr>
									<td>{owner_name}</td>
									<td>
										{#if can_write}
											<div>
												<ToggleButtonGroup
													disabled={owner_name == 'u/' + $userStore?.username &&
														!($userStore?.is_admin || $userStore?.is_super_admin)}
													selected={role}
													on:selected={async (e) => {
														const role = e.detail
														// const wasInFolder = (folder?.owners ?? []).includes(folder)
														// const inAcl = (
														// 	folder?.extra_perms ? Object.keys(folder?.extra_perms) : []
														// ).includes(folder)
														if (role == 'admin') {
															await FolderService.addOwnerToFolder({
																workspace: $workspaceStore ?? '',
																name,
																requestBody: {
																	owner: owner_name
																}
															})
														} else if (role == 'writer') {
															await FolderService.removeOwnerToFolder({
																workspace: $workspaceStore ?? '',
																name,
																requestBody: {
																	owner: owner_name,
																	write: true
																}
															})
														} else if (role == 'viewer') {
															await FolderService.removeOwnerToFolder({
																workspace: $workspaceStore ?? '',
																name,
																requestBody: {
																	owner: owner_name,
																	write: false
																}
															})
														}
														loadFolder()
													}}
												>
													{#snippet children({ item })}
														<ToggleButton
															value="viewer"
															label="Viewer"
															tooltip="A viewer of a folder has read-only access to all the elements (scripts/flows/apps/schedules/resources/variables) inside the folder"
															{item}
															size="sm"
														/>

														<ToggleButton
															value="writer"
															label="Writer"
															tooltip="A writer of a folder has read AND write access to all the elements (scripts/flows/apps/schedules/resources/variables) inside the folder"
															{item}
															size="sm"
														/>

														<ToggleButton
															value="admin"
															label="Admin"
															tooltip="An admin of a folder has read AND write access to all the elements inside the folders and can manage the permissions as well as add new admins"
															{item}
															size="sm"
														/>
													{/snippet}
												</ToggleButtonGroup>
											</div>
										{:else}
											{role}
										{/if}</td
									>
									<td class="flex items-center justify-end">
										{#if (can_write && owner_name != 'u/' + $userStore?.username) || $userStore?.is_admin}
											<Button
												variant="subtle"
												destructive
												unifiedSize="md"
												startIcon={{ icon: Trash }}
												iconOnly
												onclick={async () => {
													await Promise.all([
														FolderService.removeOwnerToFolder({
															workspace: $workspaceStore ?? '',
															name,
															requestBody: { owner: owner_name }
														}),
														GranularAclService.removeGranularAcls({
															workspace: $workspaceStore ?? '',
															path: name,
															kind: 'folder',
															requestBody: {
																owner: owner_name
															}
														})
													])

													loadFolder()
												}}
											/>
										{:else if can_write && owner_name == 'u/' + $userStore?.username}
											<span class="text-primary text-xs">cannot remove yourself</span>
										{/if}</td
									>
								</tr>{/each}
						</tbody>
					{/snippet}
				</TableCustom>
				<!-- <h2 class="mt-10"
				>Folders managing this folder <Tooltip
					>Any owner of those folders can manage this folder</Tooltip
				></h2
			>
			{#if can_write}
				<div class="flex items-start">
					<AutoComplete items={folders} bind:selectedItem={new_managing_folder} />
					<Button variant="accent" size="sm" btnClasses="!ml-4" on:click={addToManagingFolder}>
						Add folder managing this folder
					</Button>
				</div>
			{/if} -->
				<!-- {#if managing_folders.length == 0}
				<p class="text-primary text-sm">No folder is managing this folder</p>
			{:else}
				<TableCustom>
					<tr slot="headerRow">
						<th>folder</th>
						<th />
					</tr>
					<tbody slot="body">
						{#each managing_folders as managing_folder}<tr>
								<td>{managing_folder.split('/')[1]}</td>
								<td>
									{#if can_write}
										<button
											class="ml-2 text-red-500"
											on:click={async () => {
												await GranularAclService.removeGranularAcls({
													workspace: $workspaceStore ?? '',
													path: name,
													kind: 'folder',
													requestBody: {
														owner: managing_folder
													}
												})
												loadFolder()
											}}>remove</button
										>
									{/if}</td
								>
							</tr>{/each}
					</tbody>
				</TableCustom>
			{/if} -->
			{:else if folderNotFound === undefined}
				<div class="flex flex-col">
					{#each new Array(6) as _}
						<Skeleton layout={[[2], 0.7]} />
					{/each}
				</div>
			{/if}
		</div>
	</Label>

	{#if canEditDefaults}
		<CollapseLink text="Default permissioned as (advanced, prod only)">
			<div class="flex flex-col gap-2">
				<Alert type="info" title="Advanced — for prod workspaces (least privilege)" size="xs">
					This setting is mostly relevant on <strong>production workspaces</strong> where you want
					new items under this folder to run under a least-privilege service account rather than the
					deploying admin's identity. When an admin or <code>wm_deployers</code> member creates a
					trigger, schedule, app, script, or flow under this folder, the first matching rule
					determines the default <code>permissioned_as</code>. Globs are relative to the folder root
					(e.g. <code>jobs/**</code> matches <code>f/{name}/jobs/run_a</code>). Existing items are
					never rewritten.
				</Alert>

				{#if defaultPermissionedAs.length > 0}
					<TableCustom>
						{#snippet headerRow()}
							<tr>
								<th>path_glob <Tooltip>Glob relative to <code>f/{name}/</code></Tooltip></th>
								<th>permissioned as</th>
								<th class="w-24"></th>
							</tr>
						{/snippet}
						{#snippet body()}
							<tbody>
								{#each defaultPermissionedAs as rule, idx (idx)}
									{@const kind = ruleKind(rule.permissioned_as)}
									{@const itemsForKind = kind === 'user' ? usernames : groups}
									<tr>
										<td>
											<TextInput
												bind:value={rule.path_glob}
												inputProps={{ placeholder: '**' }}
												error={!isValidGlob(rule.path_glob)}
											/>
										</td>
										<td>
											<div class="flex items-center gap-1">
												<ToggleButtonGroup
													selected={kind}
													on:selected={(e) => setRulePermissionedAs(idx, e.detail, '')}
												>
													{#snippet children({ item })}
														<ToggleButton value="user" label="User" {item} size="sm" />
														<ToggleButton value="group" label="Group" {item} size="sm" />
													{/snippet}
												</ToggleButtonGroup>
												<Select
													items={safeSelectItems(itemsForKind)}
													bind:value={
														() => ruleName(rule.permissioned_as),
														(v) => setRulePermissionedAs(idx, kind, v ?? '')
													}
													class="grow min-w-32"
												/>
											</div>
										</td>
										<td>
											<div class="flex items-center gap-1 justify-end">
												<Button
													variant="subtle"
													unifiedSize="sm"
													startIcon={{ icon: ArrowUp }}
													iconOnly
													disabled={idx === 0}
													on:click={() => moveDefaultRule(idx, -1)}
												/>
												<Button
													variant="subtle"
													unifiedSize="sm"
													startIcon={{ icon: ArrowDown }}
													iconOnly
													disabled={idx === defaultPermissionedAs.length - 1}
													on:click={() => moveDefaultRule(idx, 1)}
												/>
												<Button
													variant="subtle"
													destructive
													unifiedSize="sm"
													startIcon={{ icon: Trash }}
													iconOnly
													on:click={() => removeDefaultRule(idx)}
												/>
											</div>
										</td>
									</tr>
								{/each}
							</tbody>
						{/snippet}
					</TableCustom>
				{:else}
					<div class="text-xs text-tertiary">No rules defined.</div>
				{/if}

				<div class="flex items-center gap-2">
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: Plus }}
						on:click={addDefaultRule}
					>
						Add rule
					</Button>
					<Button
						variant="accent"
						unifiedSize="sm"
						disabled={defaultRulesInvalid}
						on:click={saveDefaultRules}
					>
						Save rules
					</Button>
				</div>
			</div>
		</CollapseLink>
	{/if}

	{#if reloadHistory > 0}
		{#key reloadHistory}
			<PermissionHistory
				{name}
				fetchHistory={async (workspace, folderName, page, perPage) => {
					return await FolderService.getFolderPermissionHistory({
						workspace,
						name: folderName,
						page,
						perPage
					})
				}}
			/>
		{/key}
	{/if}
</div>
