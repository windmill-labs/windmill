<script lang="ts">
	import {
		GranularAclService,
		GroupService,
		UserService,
		type Group,
		type InstanceGroup
	} from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { onMount, tick, untrack } from 'svelte'
	import { Button } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Row from './table/Row.svelte'
	import Cell from './table/Cell.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Label from './Label.svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { Plus, Trash } from 'lucide-svelte'
	import PermissionHistory from './PermissionHistory.svelte'
	import Alert from './common/alert/Alert.svelte'
	import InputError from './InputError.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import { DEMO_RESTRICTION_HINT, isDemoWorkspaceRestricted } from '$lib/cloud'
	import {
		groupMemberDiff,
		isGroupDraftDirty,
		type GroupDraft,
		type GroupRole
	} from '$lib/groupDraft'

	const ROLE_TOOLTIPS = {
		member:
			'A Member of a group can see everything the group can see, write to everything the group can write, and generally act on behalf of the group',
		manager:
			'A manager of a group can manage the group, adding and removing users and change their roles. Being a manager does not make you a member',
		admin:
			'An admin of a group is a member of a group that can also add and remove members to the group, or make them admin.'
	}

	const MEMBERS_EXPLAINER =
		'A member is a user with a role on this group. Members act on behalf of the group and see everything it can see; admins can additionally add and remove members.'

	// Edits mutate `draft` only; `save()` is the sole writer to the backend, and `baseline` is
	// what the group held when it was loaded, so comparing the two gives both the dirty state
	// and the member calls to replay. Both live in `groupDraft.ts`, with tests.
	interface Props {
		/** In `new` mode this is the name being typed, hence bindable. */
		name: string
		mode?: 'edit' | 'new'
		/** Drives the parent drawer's Save button, which lives above this component. */
		onCanSaveChange?: (canSave: boolean) => void
		/** Drives the parent drawer's discard confirmation on close. Unlike `canSave` this
		 * stays true for edits that cannot be saved yet (a name already taken) — closing
		 * would still throw them away. */
		onUnsavedChange?: (unsaved: boolean) => void
		/** Turns true once the group exists on the server, which a `new` drawer reaches
		 * mid-save. The drawer stops calling itself Create from that point. */
		onExistsChange?: (exists: boolean) => void
	}

	let {
		name = $bindable(),
		mode = 'edit',
		onCanSaveChange,
		onUnsavedChange,
		onExistsChange
	}: Props = $props()

	const restricted = $derived(
		isDemoWorkspaceRestricted($workspaceStore, $userStore?.is_admin, $userStore?.is_super_admin)
	)

	let can_write = $state(false)
	let group: Group | undefined
	let instance_group: InstanceGroup | undefined = $state()
	let usernames: string[] = $state([])
	let groupNames: string[] = $state([])
	let loaded = $state(false)
	let reloadHistory = $state(0)
	let nameInput: TextInput | undefined = $state(undefined)

	let baseline: GroupDraft | undefined = $state(undefined)
	// Empty, not `emptyDraft()`: that one seeds the caller as an admin, which is true of a
	// group being created and a lie about one whose read failed. Every path that wants the
	// seeded row calls `emptyDraft()` itself.
	let draft: GroupDraft = $state({ summary: '', members: [] })

	let memberToAdd: string = $state('')
	let newMemberRole: GroupRole = $state('member')

	// `create_group` puts the caller in the group and gives them the write entry, so a save
	// on this branch has already happened once the request lands: a retry after a later
	// member call fails must take the edit path or it recreates a group that now exists.
	let alreadyCreated = $state(false)
	const isNew = $derived(mode === 'new' && !alreadyCreated)

	function emptyDraft(): GroupDraft {
		return {
			summary: '',
			// The backend makes the creator an admin whatever we send, so the table shows that
			// from the start rather than after the first reload.
			members: $userStore ? [{ member_name: $userStore.username, role: 'admin' as GroupRole }] : []
		}
	}

	function setDraft(value: GroupDraft) {
		baseline = structuredClone(value)
		draft = structuredClone(value)
	}

	/** Fills a picker or a validation list. The editor is usable before these land, so they
	 *  run alongside the group read — but a rejection has to be reported: unhandled, it
	 *  leaves the list silently empty and duplicate names stop being caught. */
	function loadAside(load: () => Promise<void>): void {
		load().catch((e) => sendUserToast(e?.body ?? String(e), true))
	}

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
	}

	async function loadGroupNames(): Promise<void> {
		groupNames = (await GroupService.listGroupNames({ workspace: $workspaceStore! })) ?? []
	}

	async function loadInstanceGroup(): Promise<void> {
		try {
			instance_group = await GroupService.getInstanceGroup({ name })
		} catch (e) {
			instance_group = undefined
		}
	}

	async function load() {
		loadAside(loadUsernames)
		if (isNew) {
			loadAside(loadGroupNames)
			can_write = true
			setDraft(emptyDraft())
			loaded = true
		} else {
			loadAside(loadInstanceGroup)
			await loadGroup()
		}
	}

	/** `baselineOnly` re-reads the group without touching the draft: after a save that
	 *  committed some of its calls and then failed, the baseline must become what the server
	 *  actually holds while the draft stays the user's intent — the applied changes then stop
	 *  counting as dirty, and the ones still missing stay dirty and retryable. */
	async function loadGroup(opts?: { baselineOnly?: boolean }): Promise<void> {
		const apply = (value: GroupDraft) =>
			opts?.baselineOnly ? (baseline = structuredClone(value)) : setDraft(value)
		try {
			group = await GroupService.getGroup({ workspace: $workspaceStore!, name })
			can_write = canWrite(name, group.extra_perms ?? {}, $userStore)
			apply({
				summary: group.summary ?? '',
				members: Array.from(
					new Set(
						Object.entries(group?.extra_perms ?? {})
							.filter(([k, v]) => k.startsWith('u/') && v)
							.map(([k, _]) => k.split('/')[1])
							.concat(group?.members ?? [])
					)
				).map((x) => ({ member_name: x, role: getRole(x) }))
			})
			reloadHistory++
		} catch (e) {
			// The draft must survive a failed read: overwriting it here would discard the
			// user's edits and clear `unsaved` with them.
			sendUserToast(e?.body ?? String(e), true)
			// Only the opening read decides this. Revoking it on a failed reconcile would
			// disable Save against a draft that is still dirty, with nothing left to reload.
			if (!opts?.baselineOnly) can_write = false
		} finally {
			loaded = true
		}
	}

	function getRole(x: string): GroupRole {
		const manages = 'u/' + x in (group?.extra_perms ?? {}) && (group?.extra_perms ?? {})['u/' + x]
		const belongs = group?.members?.includes(x)

		if (manages && belongs) {
			return 'admin'
		} else if (manages) {
			return 'manager'
		} else {
			return 'member'
		}
	}

	// Guarded on `isNew`, not `mode`: once the group exists the name is frozen, so there is
	// nothing left to validate.
	const nameError = $derived(
		!isNew
			? ''
			: !name
				? ''
				: groupNames.includes(name)
					? 'A group with this name already exists'
					: ''
	)

	const dirty = $derived(isGroupDraftDirty(draft, baseline))
	// A typed name is progress too, even before any other field is touched.
	const unsaved = $derived(dirty || (mode === 'new' && !!name))

	$effect(() => {
		onCanSaveChange?.(isNew ? loaded && !!name && !nameError && !restricted : can_write && dirty)
	})

	$effect(() => {
		onUnsavedChange?.(unsaved)
	})

	$effect(() => {
		onExistsChange?.(!isNew)
	})

	// `create_group` folds the caller into the group as an admin whatever the payload says, so
	// on create their own row is fixed: offering to demote or remove it would be a change the
	// backend silently discards.
	function isFixedCreatorRow(member: string): boolean {
		return isNew && member === $userStore?.username
	}

	function addMember(close: () => void) {
		if (!draft.members.some((m) => m.member_name === memberToAdd)) {
			draft.members.push({ member_name: memberToAdd, role: newMemberRole })
		}
		memberToAdd = ''
		close()
	}

	/** Replays the member rows the user changed. `updateGroup` writes the summary only, so
	 * membership goes through the endpoints that name who was added or promoted — which is
	 * what the permission history reads back. The diff itself is in `groupDraft.ts`. */
	async function applyMemberChanges(next: GroupDraft['members'], prev: GroupDraft['members']) {
		const workspace = $workspaceStore ?? ''
		for (const call of groupMemberDiff(prev, next, $userStore?.username)) {
			switch (call.kind) {
				case 'addUser':
					await GroupService.addUserToGroup({
						workspace,
						name,
						requestBody: { username: call.username }
					})
					break
				case 'removeUser':
					await GroupService.removeUserToGroup({
						workspace,
						name,
						requestBody: { username: call.username }
					})
					break
				case 'setAcl':
					await GranularAclService.addGranularAcls({
						workspace,
						path: name,
						kind: 'group_',
						requestBody: { owner: 'u/' + call.username, write: true }
					})
					break
				case 'removeAcl':
					await GranularAclService.removeGranularAcls({
						workspace,
						path: name,
						kind: 'group_',
						requestBody: { owner: 'u/' + call.username }
					})
					break
			}
		}
	}

	export async function save(): Promise<{ name: string; created: boolean } | undefined> {
		const next = $state.snapshot(draft) as GroupDraft
		const prev = baseline as GroupDraft
		const created = isNew
		try {
			if (created) {
				await GroupService.createGroup({
					workspace: $workspaceStore ?? '',
					requestBody: { name, summary: next.summary }
				})
				alreadyCreated = true
				// The members the caller added, on top of the admin row `create_group` wrote.
				await applyMemberChanges(next.members, emptyDraft().members)
				sendUserToast(`Group ${name} created`)
			} else {
				if (next.summary !== prev.summary) {
					await GroupService.updateGroup({
						workspace: $workspaceStore ?? '',
						name,
						requestBody: { summary: next.summary }
					})
				}
				await applyMemberChanges(next.members, prev.members)
				await loadGroup()
				sendUserToast('Group updated')
			}
			return { name, created }
		} catch (e) {
			sendUserToast(e.body ?? String(e), true)
			// A failed create is not proof the group is absent: `create_group` commits before a
			// git-sync step that can still fail the request, including with a 4xx. Only the name
			// conflict says it was never written. Report rather than resolve — a group found by
			// name may be someone else's, and adopting it would send this draft's writes there.
			const nameTaken = String(e?.body ?? '').includes('already exists')
			if (created && !alreadyCreated && !nameTaken) {
				sendUserToast(`Group ${name} may have been created anyway — reopen it to check`, true)
			}
			// Reconcile after any edit-path failure: the post-commit window means a rejection is
			// not proof nothing was written. The baseline moves to server truth and the draft
			// stays, so a retry re-sends only what is missing. `isNew` is read after the create,
			// so a group that now exists reconciles too.
			if (!isNew) await loadGroup({ baselineOnly: true })
			return undefined
		}
	}

	// The stores are read only to wait until they are populated, and the load runs once: this
	// editor holds an unsaved draft, and the layout re-`set`s `$userStore` periodically — a
	// second `load()` would overwrite the draft with the server's state and lose the edits
	// silently, `unsaved` included. The drawer remounts this component per opening.
	let loadStarted = false
	$effect.pre(() => {
		if (loadStarted) return
		if ($workspaceStore && $userStore) {
			loadStarted = true
			untrack(() => {
				load()
			})
		}
	})

	onMount(async () => {
		if (mode !== 'new') return
		// The editor is remounted per drawer opening, so mount is the moment the create form
		// appears; the input only exists after the first render.
		await tick()
		nameInput?.focus()
	})
</script>

<div class="flex flex-col gap-6">
	{#if name === 'wm_deployers'}
		<Alert type="info" title="Deployer permissions">
			Members of this group can preserve the original author (on_behalf_of / permissioned_as) when
			deploying scripts, flows, apps, triggers, and schedules to this workspace. Without this
			permission, deployed items will be reassigned to the deploying user.
		</Alert>
	{/if}

	{#if mode === 'new'}
		<Label label="Group name">
			<!-- Frozen once the group exists: `createGroup` lands before the member calls, so a
			     save can fail with the group already created under this name. Retyping it would
			     point the remaining member calls at a different group — one that may not exist,
			     or worse, one that does. -->
			<TextInput
				bind:this={nameInput}
				bind:value={name}
				error={!!nameError}
				size="md"
				inputProps={{ placeholder: 'group_name', disabled: !isNew }}
			/>
			<InputError error={nameError} />
		</Label>
	{/if}

	<Label label="Summary" for="summary">
		<TextInput
			inputProps={{
				placeholder: 'Short summary to be displayed when listed',
				id: 'summary',
				disabled: !can_write
			}}
			bind:value={draft.summary}
			size="md"
		/>
	</Label>

	<Label label={`Members (${draft.members.length})`} tooltip={MEMBERS_EXPLAINER}>
		{#snippet action()}
			{#if can_write && !restricted}
				<Popover
					placement="bottom-end"
					onClose={() => {
						memberToAdd = ''
						newMemberRole = 'member'
					}}
				>
					{#snippet trigger()}
						<Button
							variant="default"
							unifiedSize="sm"
							nonCaptureEvent={true}
							startIcon={{ icon: Plus }}
						>
							Add member
						</Button>
					{/snippet}
					{#snippet content({ close })}
						<div class="flex flex-col w-72 p-4 gap-4">
							<span class="text-sm leading-6 font-semibold">Add a member</span>
							<Label label="User">
								<Select
									items={safeSelectItems(
										usernames.filter((x) => !draft.members.some((m) => m.member_name === x))
									)}
									bind:value={memberToAdd}
									size="sm"
									class="grow min-w-0"
								/>
							</Label>
							<Label label="Role">
								<ToggleButtonGroup bind:selected={newMemberRole}>
									{#snippet children({ item })}
										<ToggleButton
											value="member"
											label="Member"
											tooltip={ROLE_TOOLTIPS.member}
											{item}
											size="sm"
										/>
										<ToggleButton
											value="admin"
											label="Admin"
											tooltip={ROLE_TOOLTIPS.admin}
											{item}
											size="sm"
										/>
									{/snippet}
								</ToggleButtonGroup>
							</Label>
							<Button
								variant="accent"
								unifiedSize="sm"
								disabled={memberToAdd == ''}
								onClick={() => addMember(close)}
							>
								Add
							</Button>
						</div>
					{/snippet}
				</Popover>
			{/if}
		{/snippet}
		<div class="flex flex-col gap-2">
			{#if can_write && restricted}
				<Alert type="info" title="Sharing disabled">{DEMO_RESTRICTION_HINT}</Alert>
			{/if}

			{#if loaded}
				<DataTable size="sm">
					<Head>
						<tr>
							<Cell head first class="text-secondary">Name</Cell>
							<Cell head class="text-secondary">Role</Cell>
							<Cell head last actions class="text-secondary">Actions</Cell>
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each draft.members as member, idx (member.member_name)}
							<Row>
								<Cell first>
									<span class="text-emphasis font-medium">{member.member_name}</span>
								</Cell>
								<Cell>
									{#if can_write && !restricted}
										<div>
											<ToggleButtonGroup
												disabled={isFixedCreatorRow(member.member_name)}
												selected={member.role}
												on:selected={(e) => {
													draft.members[idx].role = e.detail
												}}
											>
												{#snippet children({ item })}
													<ToggleButton
														value="member"
														label="Member"
														tooltip={ROLE_TOOLTIPS.member}
														{item}
														size="sm"
													/>
													<ToggleButton
														value="admin"
														label="Admin"
														tooltip={ROLE_TOOLTIPS.admin}
														{item}
														size="sm"
													/>
													<!-- Manager is a state the UI can leave but not enter: it is a
													     write entry without membership, which only older grants hold. -->
													{#if member.role === 'manager'}
														<ToggleButton
															value="manager"
															label="Manager"
															tooltip={ROLE_TOOLTIPS.manager}
															{item}
															size="sm"
														/>
													{/if}
												{/snippet}
											</ToggleButtonGroup>
										</div>
									{:else}
										{member.role}
									{/if}
								</Cell>
								<Cell last actions>
									<div class="flex items-center justify-end">
										{#if can_write && !isFixedCreatorRow(member.member_name)}
											<Button
												variant="subtle"
												destructive
												unifiedSize="sm"
												startIcon={{ icon: Trash }}
												iconOnly
												onclick={() => {
													draft.members = draft.members.filter(
														(m) => m.member_name !== member.member_name
													)
												}}
											/>
										{:else if isFixedCreatorRow(member.member_name)}
											<span class="text-2xs text-hint">admin as the creator</span>
										{/if}
									</div>
								</Cell>
							</Row>
						{/each}
					</tbody>
				</DataTable>
			{:else}
				<div class="flex flex-col">
					{#each new Array(6) as _}
						<Skeleton layout={[[2], 0.7]} />
					{/each}
				</div>
			{/if}
		</div>
	</Label>

	{#if instance_group?.emails}
		<Label label="Members from the instance group">
			<DataTable size="sm">
				<Head>
					<tr>
						<Cell head first last class="text-secondary">Email</Cell>
					</tr>
				</Head>
				<tbody class="divide-y">
					{#each instance_group?.emails ?? [] as email}
						<Row>
							<Cell first last>{email}</Cell>
						</Row>
					{/each}
				</tbody>
			</DataTable>
		</Label>
	{/if}

	{#if reloadHistory > 0}
		{#key reloadHistory}
			<PermissionHistory
				{name}
				fetchHistory={async (workspace, groupName, page, perPage) => {
					return await GroupService.getGroupPermissionHistory({
						workspace,
						name: groupName,
						page,
						perPage
					})
				}}
			/>
		{/key}
	{/if}
</div>
