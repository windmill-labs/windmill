<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		type Folder,
		type FolderDefaultPermissionedAs,
		FolderService,
		UserService,
		GranularAclService,
		GroupService,
		type User
	} from '$lib/gen'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Row from './table/Row.svelte'
	import Cell from './table/Cell.svelte'
	import { DEMO_RESTRICTION_HINT, isDemoWorkspaceRestricted } from '$lib/cloud'
	import { Alert, Button } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import GroupEditorDrawer from './GroupEditorDrawer.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { ArrowDown, ArrowUp, Eye, Pen, Plus, Trash } from 'lucide-svelte'
	import Label from './Label.svelte'
	import { sendUserToast } from '$lib/toast'
	import { onMount, tick, untrack } from 'svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import PermissionHistory from './PermissionHistory.svelte'
	import { Minimatch } from 'minimatch'
	import Tooltip from './Tooltip.svelte'
	import CollapseLink from './CollapseLink.svelte'
	import LabelsInput from './LabelsInput.svelte'
	import Badge from './common/badge/Badge.svelte'
	import InputError from './InputError.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import { deepEqual } from 'fast-equals'
	import {
		folderPermissionDiff,
		isFolderDraftDirty,
		type FolderDraft,
		type FolderRole
	} from '$lib/folderDraft'

	const VALID_FOLDER_NAME = /^[a-zA-Z_0-9-]+$/

	const ROLE_TOOLTIPS = {
		viewer:
			'A viewer of a folder has read-only access to all the elements (scripts/flows/apps/schedules/resources/variables) inside the folder',
		writer:
			'A writer of a folder has read AND write access to all the elements (scripts/flows/apps/schedules/resources/variables) inside the folder',
		admin:
			'An admin of a folder has read AND write access to all the elements inside the folders and can manage the permissions as well as add new admins'
	}

	const MEMBERS_EXPLAINER =
		"A member is a user or group with a role on this folder. The role applies to every script, flow, app, resource, variable and schedule inside it: viewers can read them, writers can also edit them, and admins can additionally manage the folder's members."

	// Edits mutate `draft` only; `save()` is the sole writer to the backend, and `baseline` is
	// what the folder held when it was loaded, so comparing the two gives both the dirty state
	// and the permission calls to replay. Both live in `folderDraft.ts`, with tests.
	type Role = FolderRole

	interface Props {
		/** In `new` mode this is the name being typed, hence bindable. */
		name: string
		mode?: 'edit' | 'new'
		/** Drives the parent drawer's Save button, which lives above this component. */
		onCanSaveChange?: (canSave: boolean) => void
		/** Drives the parent drawer's discard confirmation on close. Unlike `canSave`
		 * this stays true for edits that cannot be saved yet (an invalid rule, a name
		 * already taken) — closing would still throw them away. */
		onUnsavedChange?: (unsaved: boolean) => void
		/** False while Save would create rather than update, which an `edit` drawer reaches
		 * when the folder turns out not to exist. The drawer labels itself from this. */
		onExistsChange?: (exists: boolean) => void
		/** Edit a folder of this workspace rather than the active one. The folder picker
		 * can be aimed elsewhere (the project import wizard picks a destination workspace
		 * before entering it), and the folder must be written where it was listed. */
		workspace?: string
	}

	let {
		name = $bindable(),
		mode = 'edit',
		onCanSaveChange,
		onUnsavedChange,
		onExistsChange,
		workspace
	}: Props = $props()

	const targetWorkspace = $derived(workspace ?? $workspaceStore ?? '')
	const aimedElsewhere = $derived(!!workspace && workspace !== $workspaceStore)

	// `$userStore` describes the workspace the app is *in*. Aimed at another one it answers
	// the wrong question — a folder admin there would get read-only controls, and a
	// non-member would get write ones — so resolve the membership of the workspace being
	// edited. `whoami` returns group names unprefixed; `owners` holds them `g/`-prefixed.
	let targetUser: User | undefined = $state(undefined)
	const membership = $derived.by(() => {
		if (!aimedElsewhere) {
			return $userStore
				? {
						username: $userStore.username,
						is_admin: $userStore.is_admin ?? false,
						is_super_admin: $userStore.is_super_admin ?? false,
						pgroups: $userStore.pgroups ?? [],
						groups: $userStore.groups ?? []
					}
				: undefined
		}
		return targetUser
			? {
					username: targetUser.username,
					is_admin: targetUser.is_admin ?? false,
					is_super_admin: targetUser.is_super_admin ?? false,
					pgroups: (targetUser.groups ?? []).map((g) => 'g/' + g),
					groups: targetUser.groups ?? []
				}
			: undefined
	})

	async function loadTargetUser(): Promise<void> {
		if (!aimedElsewhere || !workspace) return
		try {
			targetUser = await UserService.whoami({ workspace })
		} catch {
			// Not a member, or the call failed: no membership means read-only controls,
			// which is the safe reading — the write would be refused anyway.
			targetUser = undefined
		}
	}

	let can_write = $state(false)
	let folder: Folder | undefined
	let usernames: string[] = $state([])
	let groups: string[] = $state([])
	let folderNames: string[] = $state([])
	let ownerItem: string = $state('')

	let groupEditorDrawer: GroupEditorDrawer | undefined = $state(undefined)
	let addMemberPopover: Popover | undefined = $state(undefined)
	let nameInput: TextInput | undefined = $state(undefined)

	let baseline: FolderDraft | undefined = $state(undefined)
	// Empty, not `emptyDraft()`: that one seeds the caller as an admin, which is true of a
	// folder being created and a lie about one whose read failed. Every path that wants the
	// seeded row calls `emptyDraft()` itself.
	let draft: FolderDraft = $state({
		summary: '',
		labels: [],
		defaultPermissionedAs: [],
		perms: []
	})
	let labelsInput: LabelsInput | undefined = $state()
	let pendingLabel = $state('')
	let folderNotFound: boolean | undefined = $state(undefined)
	let loaded = $state(false)

	// A name typed in `new` mode, and one whose folder turned out not to exist, both
	// end up at `createFolder` on save.
	const isNew = $derived(mode === 'new' || folderNotFound === true)

	function emptyDraft(): FolderDraft {
		return {
			summary: '',
			labels: [],
			defaultPermissionedAs: [],
			// The backend makes the creator an owner whatever we send, so the table
			// shows that from the start rather than after the first reload.
			perms: membership ? [{ owner_name: 'u/' + membership.username, role: 'admin' as Role }] : []
		}
	}

	function setDraft(value: FolderDraft) {
		baseline = structuredClone(value)
		draft = structuredClone(value)
	}

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: targetWorkspace })
	}

	async function loadGroups(): Promise<void> {
		groups = await GroupService.listGroupNames({ workspace: targetWorkspace })
	}

	async function loadFolderNames(): Promise<void> {
		folderNames = await FolderService.listFolderNames({ workspace: targetWorkspace })
	}

	/** Fills a picker or a validation list. The editor is usable before these land, so they
	 *  run alongside the folder read — but a rejection has to be reported: unhandled, it
	 *  leaves the list silently empty and duplicate names stop being caught. */
	function loadAside(load: () => Promise<void>): void {
		load().catch((e) => sendUserToast(e?.body ?? String(e), true))
	}

	async function load() {
		loadAside(loadUsernames)
		loadAside(loadGroups)
		// Before the folder read: `can_write` is computed from this membership.
		await loadTargetUser()
		if (mode === 'new') {
			loadAside(loadFolderNames)
			can_write = true
			setDraft(emptyDraft())
			loaded = true
		} else {
			await loadFolder()
		}
	}

	function grant(close: () => void) {
		const owner = (ownerKind == 'user' ? 'u/' : 'g/') + ownerItem
		if (!draft.perms.some((p) => p.owner_name === owner)) {
			draft.perms.push({ owner_name: owner, role: newMemberRole })
		}
		ownerItem = ''
		close()
	}

	/** `baselineOnly` re-reads the folder without touching the draft: after a save that
	 *  committed some of its calls and then failed, the baseline must become what the server
	 *  actually holds while the draft stays the user's intent — the applied changes then stop
	 *  counting as dirty, and the ones still missing stay dirty and retryable. */
	async function loadFolder(opts?: { baselineOnly?: boolean }): Promise<void> {
		const apply = (value: FolderDraft) =>
			opts?.baselineOnly ? (baseline = structuredClone(value)) : setDraft(value)
		try {
			folder = await FolderService.getFolder({ workspace: targetWorkspace, name })
			folderNotFound = false
			can_write =
				membership != undefined &&
				(folder?.owners.includes('u/' + membership.username) ||
					membership.is_admin ||
					membership.is_super_admin ||
					membership.pgroups.findIndex((x) => folder?.owners.includes(x)) != -1)

			apply({
				summary: folder.summary ?? '',
				labels: [...(folder.labels ?? [])],
				defaultPermissionedAs: (folder.default_permissioned_as ?? []).map((r) => ({ ...r })),
				perms: Array.from(
					new Set(
						Object.entries(folder?.extra_perms ?? {})
							.map((x) => x[0])
							.concat(folder?.owners ?? [])
					)
				).map((x) => ({ owner_name: x, role: getRole(x) }))
			})
			reloadHistory++
		} catch (e) {
			// Only a folder that is genuinely absent may replace the draft — it can be created
			// from here, so the editor opens on an empty one rather than a dead end. Any other
			// failure (network, 5xx) must leave the draft alone: overwriting it here would
			// discard the user's edits and clear `unsaved` with them.
			if (e?.status === 404) {
				folderNotFound = true
				can_write = true
				apply(emptyDraft())
			} else {
				sendUserToast(e?.body ?? String(e), true)
			}
		} finally {
			loaded = true
		}
	}

	const restricted = $derived(
		isDemoWorkspaceRestricted(targetWorkspace, membership?.is_admin, membership?.is_super_admin)
	)

	const canEditDefaults = $derived(
		can_write &&
			!restricted &&
			(membership?.is_admin ||
				membership?.is_super_admin ||
				(membership?.groups ?? []).includes('wm_deployers'))
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

	// Split an owner value like "u/alice" or "g/prod" into its kind and name.
	function ownerKindOf(value: string): 'user' | 'group' {
		return value.startsWith('g/') ? 'group' : 'user'
	}
	function ownerNameOf(value: string): string {
		if (value.startsWith('u/') || value.startsWith('g/')) return value.slice(2)
		return value
	}
	function setRulePermissionedAs(idx: number, kind: 'user' | 'group', name: string) {
		const prefix = kind === 'user' ? 'u/' : 'g/'
		draft.defaultPermissionedAs[idx].permissioned_as = prefix + name
	}

	// Only blocks a save for someone who can see the rules. The backend accepts values this
	// rejects (`u/` alone passes `validate_default_permissioned_as`), so a folder admin who
	// is not a workspace admin could otherwise meet a permanently disabled Save with no rule
	// on screen to explain it.
	const defaultRulesInvalid = $derived(
		canEditDefaults &&
			draft.defaultPermissionedAs.some(
				(r) => !isValidGlob(r.path_glob) || !isValidPermissionedAs(r.permissioned_as)
			)
	)

	function addDefaultRule() {
		draft.defaultPermissionedAs = [
			...draft.defaultPermissionedAs,
			{ path_glob: '**', permissioned_as: '' }
		]
	}

	function removeDefaultRule(idx: number) {
		draft.defaultPermissionedAs = draft.defaultPermissionedAs.filter((_, i) => i !== idx)
	}

	function moveDefaultRule(idx: number, delta: -1 | 1) {
		const next = [...draft.defaultPermissionedAs]
		const target = idx + delta
		if (target < 0 || target >= next.length) return
		;[next[idx], next[target]] = [next[target], next[idx]]
		draft.defaultPermissionedAs = next
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
	let newMemberRole: Role = $state('viewer')

	// Set when the group editor is opened from the add-member form, so that saving returns
	// there. Opened from a member row instead, that group is already a member and reopening
	// the form on it would offer to add it twice.
	let groupEditorInterruptedPicker = false

	function openGroupEditor(groupName: string, fromPicker: boolean) {
		groupEditorInterruptedPicker = fromPicker
		if (groupName) groupEditorDrawer?.initEdit(groupName)
		else groupEditorDrawer?.initNew()
	}

	async function onGroupSaved(groupName: string) {
		// The group has to be in `groups` before the picker reopens, or the value set below
		// has no matching item to show.
		try {
			await loadGroups()
		} catch (e) {
			sendUserToast(e?.body ?? String(e), true)
		}
		if (!groupEditorInterruptedPicker) return
		// Editing a group was a detour from adding a member: come back to the form on that
		// group so the interrupted job can be finished.
		ownerKind = 'group'
		ownerItem = groupName
		addMemberPopover?.open()
	}

	// Guarded on `mode`, not `isNew`: the name field is rendered only in `new` mode, so on the
	// not-found branch there is no input to annotate and no name the user could correct.
	const nameError = $derived(
		mode !== 'new'
			? ''
			: !name
				? ''
				: !VALID_FOLDER_NAME.test(name)
					? 'Folder name can only contain alphanumeric characters, underscores, and hyphens'
					: folderNames.includes(name)
						? 'A folder with this name already exists'
						: ''
	)

	// `create_folder` folds the caller into `owners` with write whatever the payload says, so
	// on create their own row is fixed: offering to demote or remove it would be a change the
	// backend silently discards.
	// An invalid rule disables Save, so the section holding it is held open rather than merely
	// opened once: collapsing it would hide the only explanation for the disabled button.
	let defaultRulesOpen = $state(false)

	function isFixedCreatorRow(owner: string): boolean {
		return isNew && owner === 'u/' + membership?.username
	}

	// The label input holds typed text until Enter or a blur, and that text is an edit like
	// any other: it has to count as dirty here, or Save stays disabled when it is the only
	// change and closing drops it without asking. `save()` flushes it into `draft.labels`.
	const dirty = $derived(isFolderDraftDirty(draft, baseline) || pendingLabel !== '')
	// A typed name is progress too, even before any other field is touched.
	const unsaved = $derived(dirty || (mode === 'new' && !!name))

	$effect(() => {
		onCanSaveChange?.(
			isNew
				? loaded && !!name && !nameError && !restricted && !defaultRulesInvalid
				: can_write && dirty && !defaultRulesInvalid
		)
	})

	$effect(() => {
		onUnsavedChange?.(unsaved)
	})

	$effect(() => {
		onExistsChange?.(!isNew)
	})

	/** Replays the permission rows the user changed. `updateFolder` could write
	 * `owners`/`extra_perms` wholesale in the same call as the settings, but it only
	 * logs a single "update owners"/"update acl" entry, so the permission history
	 * would stop naming who was granted what. The diff itself is in `folderDraft.ts`. */
	async function applyPermissionChanges(next: FolderDraft['perms'], prev: FolderDraft['perms']) {
		const workspace = targetWorkspace
		const callerOwners = membership
			? ['u/' + membership.username, ...(membership.pgroups ?? [])]
			: []
		for (const call of folderPermissionDiff(prev, next, callerOwners)) {
			switch (call.kind) {
				case 'grantAdmin':
					await FolderService.addOwnerToFolder({
						workspace,
						name,
						requestBody: { owner: call.owner }
					})
					break
				case 'demoteAdmin':
					await FolderService.removeOwnerToFolder({
						workspace,
						name,
						requestBody: { owner: call.owner, write: call.write }
					})
					break
				case 'setAcl':
					await GranularAclService.addGranularAcls({
						workspace,
						path: name,
						kind: 'folder',
						requestBody: { owner: call.owner, write: call.write }
					})
					break
				case 'remove':
					// Sequential, and `removeowner` first: the write policy refuses it when the
					// member being removed is the caller's last admin handle. Failing there leaves
					// the folder untouched, where the other order strands a member with no grant
					// but still in `owners`.
					await FolderService.removeOwnerToFolder({
						workspace,
						name,
						requestBody: { owner: call.owner }
					})
					await GranularAclService.removeGranularAcls({
						workspace,
						path: name,
						kind: 'folder',
						requestBody: { owner: call.owner }
					})
					break
			}
		}
	}

	export async function save(): Promise<{ name: string; created: boolean } | undefined> {
		// Clicking Save blurs the label input, which commits its text on a delay — after the
		// snapshot below. Take the label first or it is dropped as the drawer closes.
		labelsInput?.flushPendingLabel()
		const next = $state.snapshot(draft) as FolderDraft
		const prev = baseline as FolderDraft
		// Captured before the write: an edit-branch save reloads, which clears `folderNotFound`.
		const created = isNew
		try {
			if (created) {
				await FolderService.createFolder({
					workspace: targetWorkspace,
					requestBody: {
						name,
						summary: next.summary,
						labels: next.labels,
						default_permissioned_as: next.defaultPermissionedAs,
						owners: next.perms.filter((p) => p.role === 'admin').map((p) => p.owner_name),
						extra_perms: Object.fromEntries(
							next.perms.map((p) => [p.owner_name, p.role !== 'viewer'])
						)
					}
				})
				sendUserToast(`Folder ${name} created`)
			} else {
				const requestBody: {
					summary?: string
					labels?: string[]
					default_permissioned_as?: FolderDefaultPermissionedAs
				} = {}
				if (next.summary !== prev.summary) requestBody.summary = next.summary
				if (!deepEqual(next.labels, prev.labels)) requestBody.labels = next.labels
				if (!deepEqual(next.defaultPermissionedAs, prev.defaultPermissionedAs)) {
					requestBody.default_permissioned_as = next.defaultPermissionedAs
				}
				if (Object.keys(requestBody).length > 0) {
					await FolderService.updateFolder({ workspace: targetWorkspace, name, requestBody })
				}
				await applyPermissionChanges(next.perms, prev.perms)
				await loadFolder()
				sendUserToast('Folder updated')
			}
			return { name, created }
		} catch (e) {
			sendUserToast(e.body ?? String(e), true)
			// A failed create is not proof the folder is absent: `create_folder` commits before a
			// git-sync step that can still fail the request. Only the name conflict says it was
			// never written. Report rather than resolve — a folder found by name may be someone
			// else's, and adopting it would send this draft's writes there.
			const nameTaken = String(e?.body ?? '').includes('already exists')
			if (created && !nameTaken) {
				sendUserToast(`Folder ${name} may have been created anyway — reopen it to check`, true)
			}
			// Reconcile after any edit-path failure rather than tracking which calls landed:
			// these handlers commit before a git-sync step that can still fail the request, so
			// a rejection is not proof nothing was written. The baseline moves to what the
			// server now holds and the draft stays, so a retry re-sends only what is missing.
			if (!created) await loadFolder({ baselineOnly: true })
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

	let reloadHistory = $state(0)

	onMount(async () => {
		if (mode !== 'new') return
		// The editor is remounted per drawer opening, so mount is the moment the
		// create form appears; the input only exists after the first render.
		await tick()
		nameInput?.focus()
	})
</script>

<GroupEditorDrawer bind:this={groupEditorDrawer} onSaved={onGroupSaved} />

<div class="flex flex-col gap-6">
	{#if mode === 'new'}
		<Label label="Folder name">
			<TextInput
				bind:this={nameInput}
				bind:value={name}
				error={!!nameError}
				size="md"
				inputProps={{ placeholder: 'folder_name' }}
			/>
			<InputError error={nameError} />
		</Label>
	{/if}

	<Label label="Summary">
		<TextInput
			inputProps={{
				placeholder: 'Short summary to be displayed when listed',
				disabled: !can_write
			}}
			bind:value={draft.summary}
			size="md"
		/>
	</Label>

	<Label label="Labels">
		<div class="flex flex-col gap-1">
			<div class="text-xs text-tertiary">
				Scripts and flows inside this folder inherit these labels, and runs of items in this folder
				are labeled with them.
			</div>
			{#if can_write}
				<LabelsInput
					bind:this={labelsInput}
					bind:labels={draft.labels}
					workspace={targetWorkspace}
					onPendingChange={(v) => (pendingLabel = v)}
				/>
			{:else}
				<div class="inline-flex items-center gap-1 h-5">
					{#each draft.labels as label (label)}
						<Badge color="blue" small>{label}</Badge>
					{:else}
						<span class="text-xs text-tertiary">No labels</span>
					{/each}
				</div>
			{/if}
		</div>
	</Label>

	<Label label={`Members (${draft.perms.length})`} tooltip={MEMBERS_EXPLAINER}>
		{#snippet action()}
			{#if can_write && !restricted}
				<Popover
					bind:this={addMemberPopover}
					placement="bottom-end"
					onClose={() => {
						ownerItem = ''
						newMemberRole = 'viewer'
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
							<Label label="User or group">
								<div class="flex items-center gap-1">
									<!-- The toggle group is `w-full`; unwrapped it takes half the row. -->
									<div>
										<ToggleButtonGroup
											bind:selected={ownerKind}
											on:selected={() => (ownerItem = '')}
										>
											{#snippet children({ item })}
												<ToggleButton value="user" label="User" {item} size="sm" />
												<ToggleButton value="group" label="Group" {item} size="sm" />
											{/snippet}
										</ToggleButtonGroup>
									</div>

									{#key ownerKind}
										{@const items =
											ownerKind === 'user'
												? usernames.filter(
														(x) => !draft.perms.some((y) => y.owner_name === 'u/' + x)
													)
												: groups.filter((x) => !draft.perms.some((y) => y.owner_name === 'g/' + x))}
										<Select
											items={safeSelectItems(items)}
											bind:value={ownerItem}
											size="sm"
											class="grow min-w-0"
										>
											{#snippet endSnippet({ item, close: closeSelect })}
												<!-- GroupEditor reads and writes `$workspaceStore` and takes no workspace of its
												     own, so it cannot follow a drawer aimed at another one: viewing a group
												     there would edit the same-named group in the active workspace. -->
												{#if ownerKind == 'group' && !aimedElsewhere}
													<Button
														title="View group"
														variant="subtle"
														unifiedSize="xs"
														wrapperClasses="-mr-2 pl-1 -my-2"
														btnClasses="hover:bg-surface-tertiary"
														onClick={() => {
															closeSelect()
															close()
															openGroupEditor(item.value ?? '', true)
														}}
														startIcon={{ icon: Eye }}
														iconOnly
													/>
												{/if}
											{/snippet}
											{#snippet bottomSnippet({ close: closeSelect })}
												{#if ownerKind == 'group' && !aimedElsewhere}
													<Button
														variant="subtle"
														unifiedSize="sm"
														startIcon={{ icon: Plus }}
														wrapperClasses="border-t border-border-light"
														btnClasses="w-full rounded-none font-medium"
														onClick={() => {
															closeSelect()
															close()
															openGroupEditor('', true)
														}}
													>
														New group
													</Button>
												{/if}
											{/snippet}
										</Select>
									{/key}
								</div>
							</Label>
							<Label label="Role">
								<ToggleButtonGroup bind:selected={newMemberRole}>
									{#snippet children({ item })}
										<ToggleButton
											value="viewer"
											label="Viewer"
											tooltip={ROLE_TOOLTIPS.viewer}
											{item}
											size="sm"
										/>
										<ToggleButton
											value="writer"
											label="Writer"
											tooltip={ROLE_TOOLTIPS.writer}
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
							<div class="flex flex-col gap-1">
								<Button
									variant="accent"
									unifiedSize="sm"
									disabled={ownerItem == ''}
									onClick={() => grant(close)}
								>
									Add
								</Button>
								<span class="text-2xs text-hint">
									New permissions may take up to 60s to apply, due to permissions cache
									invalidation.
								</span>
							</div>
						</div>
					{/snippet}
				</Popover>
			{/if}
		{/snippet}
		<div class="flex flex-col gap-2">
			{#if can_write && restricted}
				<Alert type="info" title="Sharing disabled">{DEMO_RESTRICTION_HINT}</Alert>
			{/if}

			{#if folderNotFound}
				<Alert type="warning" title="Folder not found" size="xs">
					The folder "{name}" does not exist in the workspace. Saving will create it. An item can
					seemingly be in a folder given its path without the folder existing. A windmill folder has
					settable permissions that its children inherit. If an item is within a non-existing
					folders, only admins will see it.
				</Alert>
			{/if}
			{#if loaded}
				<DataTable size="sm">
					<Head>
						<tr>
							<Cell head first class="text-secondary">Name</Cell>
							<Cell head class="text-secondary">Kind</Cell>
							<Cell head class="text-secondary">Role</Cell>
							<Cell head last actions class="text-secondary">Actions</Cell>
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each draft.perms as perm, idx (perm.owner_name)}
							<Row>
								<Cell first>
									<span class="text-emphasis font-medium">{ownerNameOf(perm.owner_name)}</span>
								</Cell>
								<Cell>{ownerKindOf(perm.owner_name) === 'group' ? 'Group' : 'User'}</Cell>
								<Cell>
									{#if can_write && !restricted}
										<div>
											<ToggleButtonGroup
												disabled={isFixedCreatorRow(perm.owner_name) ||
													(perm.owner_name == 'u/' + membership?.username &&
														!(membership?.is_admin || membership?.is_super_admin))}
												selected={perm.role}
												on:selected={(e) => {
													draft.perms[idx].role = e.detail
												}}
											>
												{#snippet children({ item })}
													<ToggleButton
														value="viewer"
														label="Viewer"
														tooltip={ROLE_TOOLTIPS.viewer}
														{item}
														size="sm"
													/>

													<ToggleButton
														value="writer"
														label="Writer"
														tooltip={ROLE_TOOLTIPS.writer}
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
										</div>
									{:else}
										{perm.role}
									{/if}
								</Cell>
								<Cell last actions>
									<div class="flex items-center justify-end">
										<!-- The group editor reads `$workspaceStore`, so it can only be opened for the
										     workspace the app is in — see the picker's own buttons. It decides on its
										     own whether the group is editable here; a member with no write on it still
										     gets to see who is in it. -->
										{#if ownerKindOf(perm.owner_name) === 'group' && !aimedElsewhere}
											<Button
												title="Manage group"
												variant="subtle"
												unifiedSize="sm"
												startIcon={{ icon: Pen }}
												iconOnly
												onclick={() => openGroupEditor(ownerNameOf(perm.owner_name), false)}
											/>
										{/if}
										{#if !isFixedCreatorRow(perm.owner_name) && ((can_write && perm.owner_name != 'u/' + membership?.username) || membership?.is_admin)}
											<Button
												variant="subtle"
												destructive
												unifiedSize="sm"
												startIcon={{ icon: Trash }}
												iconOnly
												onclick={() => {
													draft.perms = draft.perms.filter((p) => p.owner_name !== perm.owner_name)
												}}
											/>
										{:else if can_write && perm.owner_name == 'u/' + membership?.username}
											<span class="text-2xs text-hint">
												{isFixedCreatorRow(perm.owner_name)
													? 'admin as the creator'
													: 'cannot remove yourself'}
											</span>
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

	{#if canEditDefaults}
		<CollapseLink
			bind:open={() => defaultRulesOpen || defaultRulesInvalid, (v) => (defaultRulesOpen = v)}
			text="Default permissioned as (advanced, prod only)"
		>
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

				{#if draft.defaultPermissionedAs.length > 0}
					<DataTable size="sm">
						<Head>
							<tr>
								<Cell head first class="text-secondary">
									path_glob <Tooltip>Glob relative to <code>f/{name}/</code></Tooltip>
								</Cell>
								<Cell head class="text-secondary">Permissioned as</Cell>
								<Cell head last actions class="text-secondary">Actions</Cell>
							</tr>
						</Head>
						<tbody class="divide-y">
							{#each draft.defaultPermissionedAs as rule, idx (idx)}
								{@const kind = ownerKindOf(rule.permissioned_as)}
								{@const itemsForKind = kind === 'user' ? usernames : groups}
								<Row>
									<Cell first>
										<TextInput
											bind:value={rule.path_glob}
											size="sm"
											inputProps={{ placeholder: '**' }}
											error={!isValidGlob(rule.path_glob)}
										/>
									</Cell>
									<Cell>
										<div class="flex items-center gap-1">
											<div>
												<ToggleButtonGroup
													selected={kind}
													on:selected={(e) => setRulePermissionedAs(idx, e.detail, '')}
												>
													{#snippet children({ item })}
														<ToggleButton value="user" label="User" {item} size="sm" />
														<ToggleButton value="group" label="Group" {item} size="sm" />
													{/snippet}
												</ToggleButtonGroup>
											</div>
											<Select
												items={safeSelectItems(itemsForKind)}
												size="sm"
												error={!isValidPermissionedAs(rule.permissioned_as)}
												bind:value={
													() => ownerNameOf(rule.permissioned_as),
													(v) => setRulePermissionedAs(idx, kind, v ?? '')
												}
												class="grow min-w-0"
											/>
										</div>
									</Cell>
									<Cell last actions>
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
												disabled={idx === draft.defaultPermissionedAs.length - 1}
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
									</Cell>
								</Row>
							{/each}
						</tbody>
					</DataTable>
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
				</div>
			</div>
		</CollapseLink>
	{/if}

	<!-- PermissionHistory fetches against `$workspaceStore`; aimed elsewhere it would show
	     another folder's history entirely. -->
	{#if !isNew && !aimedElsewhere && reloadHistory > 0}
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
