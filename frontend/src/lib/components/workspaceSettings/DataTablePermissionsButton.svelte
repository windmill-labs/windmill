<script lang="ts">
	import { Button } from '../common'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import Checkbox from '../common/checkbox/Checkbox.svelte'
	import Cell from '../table/Cell.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import { KeyRound, Plus } from 'lucide-svelte'
	import {
		FolderService,
		GroupService,
		UserService,
		WorkspaceService,
		type DatatableRoleInfo
	} from '$lib/gen'
	import { userWorkspaces } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { randomUUID } from '$lib/utils/uuid'
	import { ADMIN_DATATABLE_ROLE } from '../dbTypes'
	import PgAclEditor from '../datatableAcl/PgAclEditor.svelte'
	import { resource } from 'runed'
	import { deepEqual } from 'fast-equals'

	let {
		workspace,
		datatable,
		disabled = false,
		hideTrigger = false,
		onSaved
	}: {
		workspace: string
		datatable: string
		disabled?: boolean
		/** Mount the drawer without its button, so a caller can drive it via `open()`
		 * (the database manager opens it from the tree's row menu). */
		hideTrigger?: boolean
		/** Called once the roles have actually changed, so a caller showing them
		 * (the manager's role picker) can read them again. */
		onSaved?: () => void
	} = $props()

	// The server refuses to enable permissions in a fork: its data table points
	// either at the database of the workspace it was forked from, where the roles
	// would be invisible to that workspace's own config, or at a copy the fork can
	// drop. Turning them off stays available, so one that already has them can be
	// rid of them.
	const isFork = $derived(
		!!$userWorkspaces.find((w) => w.id === workspace)?.parent_workspace_id
	)

	// Matches every workspace member, unlike the `all` group whose membership is
	// bookkeeping that can drift.
	const WILDCARD_TENANT = '*'

	// Stable client-side id so a rename (A -> B) is sent as a rename rather than
	// read as a delete plus an add, which would drop the role's grants.
	type EditedRole = { id: string; name: string; tenants: string[]; pg_rolename?: string }

	let open = $state(false)
	let loading = $state(false)
	let saving = $state(false)
	let loadError = $state<string | undefined>(undefined)

	let enabled = $state(false)
	let roles = $state<EditedRole[]>([])
	/** Tracked by id, not name, so renaming the default role keeps it selected. */
	let defaultRoleId = $state<string | undefined>(undefined)
	/** The last loaded state, to diff renames and detect unsaved changes against. */
	let saved = $state<{ enabled: boolean; roles: EditedRole[]; defaultRoleId?: string }>({
		enabled: false,
		roles: []
	})

	const tenantItems = resource([() => workspace], async ([ws]) => {
		if (!ws) return []
		const [users, groups, folders] = await Promise.all([
			UserService.listUsernames({ workspace: ws }),
			GroupService.listGroupNames({ workspace: ws }),
			FolderService.listFolderNames({ workspace: ws })
		])
		return [
			{ value: WILDCARD_TENANT, label: 'Everyone', group: 'Anyone in the workspace' },
			...users.map((u) => ({ value: `u/${u}`, label: u, group: 'Users' })),
			...groups.map((g) => ({ value: `g/${g}`, label: g, group: 'Groups' })),
			...folders.map((f) => ({ value: `f/${f}`, label: f, group: 'Folders' }))
		]
	})

	function toEdited(role: DatatableRoleInfo): EditedRole {
		return {
			id: randomUUID(),
			name: role.name,
			tenants: [...(role.tenants ?? [])],
			pg_rolename: role.pg_rolename
		}
	}

	async function load() {
		loading = true
		loadError = undefined
		try {
			const res = await WorkspaceService.getDatatablePermissions({
				workspace,
				datatableName: datatable
			})
			// The backend returns roles in name order; admin leads the list instead,
			// since it is the one every other role is defined against.
			const loaded = res.roles
				.map(toEdited)
				.sort(
					(a, b) =>
						Number(b.name === ADMIN_DATATABLE_ROLE) - Number(a.name === ADMIN_DATATABLE_ROLE)
				)
			// A data table that has never been opted in comes back with no roles;
			// showing admin straight away is what the toggle is about to create.
			if (!loaded.some((r) => r.name === ADMIN_DATATABLE_ROLE)) {
				loaded.unshift({ id: randomUUID(), name: ADMIN_DATATABLE_ROLE, tenants: [] })
			}
			enabled = res.enabled
			roles = loaded
			defaultRoleId = loaded.find((r) => r.name === res.default_role)?.id ?? loaded[0]?.id
			saved = {
				enabled: res.enabled,
				roles: structuredClone($state.snapshot(loaded)),
				defaultRoleId
			}
		} catch (e) {
			loadError = e?.body ?? e?.message ?? String(e)
		} finally {
			loading = false
		}
	}

	function openDrawer() {
		open = true
		load()
	}

	/** Open the drawer without the trigger button (the database manager's tree
	 * row menu drives it). Not named `open`: that is the drawer's own state. */
	export function openPermissions() {
		openDrawer()
	}

	function addRole() {
		roles.push({ id: randomUUID(), name: '', tenants: [] })
	}

	function removeRole(id: string) {
		roles = roles.filter((r) => r.id !== id)
		if (defaultRoleId === id) {
			// Deleting the default falls back to admin rather than leaving the save
			// pointing at a role that no longer exists.
			defaultRoleId = roles.find((r) => r.name === ADMIN_DATATABLE_ROLE)?.id
		}
	}

	let hasUnsavedChanges = $derived(
		!deepEqual(saved, {
			enabled,
			roles: $state.snapshot(roles) as EditedRole[],
			defaultRoleId
		})
	)

	// The backend validates these too; catching them here keeps a half-typed row
	// from costing a round trip to find out.
	let nameError = $derived.by(() => {
		if (!enabled) return undefined
		const names = roles.map((r) => r.name.trim())
		if (names.some((n) => !n)) return 'Every role needs a name'
		const invalid = names.find((n) => !/^[a-zA-Z0-9_-]{1,63}$/.test(n))
		if (invalid) return `Invalid role name '${invalid}': letters, digits, '_' or '-' only`
		if (new Set(names).size !== names.length) return 'Role names must be unique'
		return undefined
	})

	function buildRequest() {
		const savedById = new Map(saved.roles.map((r) => [r.id, r.name]))
		return {
			enabled,
			roles: roles.map((r) => ({ name: r.name.trim(), tenants: $state.snapshot(r.tenants) })),
			default_role: roles.find((r) => r.id === defaultRoleId)?.name.trim() ?? ADMIN_DATATABLE_ROLE,
			renames: roles
				.filter((r) => savedById.has(r.id) && savedById.get(r.id) !== r.name.trim())
				.map((r) => ({ from: savedById.get(r.id)!, to: r.name.trim() }))
		}
	}

	let preview = $state<{ statements: string[]; warnings: string[] } | undefined>(undefined)
	let applying = $state(false)

	// Save is always a two-step: the SQL a change plans out is shown before it is
	// run, since creating and especially dropping roles is not undoable.
	async function requestPreview() {
		saving = true
		try {
			preview = await WorkspaceService.previewDatatablePermissions({
				workspace,
				datatableName: datatable,
				requestBody: buildRequest()
			})
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			saving = false
		}
	}

	async function apply() {
		applying = true
		try {
			await WorkspaceService.setDatatablePermissions({
				workspace,
				datatableName: datatable,
				requestBody: buildRequest()
			})
			sendUserToast('Data table permissions saved')
			preview = undefined
			await load()
			onSaved?.()
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			applying = false
		}
	}
</script>

{#if !hideTrigger}
	<Button
		unifiedSize="xs"
		variant="default"
		startIcon={{ icon: KeyRound }}
		iconOnly
		{disabled}
		title="Permissions: restrict who can use this data table, and as which database role"
		on:click={openDrawer}
	/>
{/if}

<Drawer bind:open size="900px">
	<DrawerContent
		title="Permissions — {datatable}"
		on:close={() => (open = false)}
		tooltip="Map Windmill roles onto Postgres roles, so a script that runs as a role connects as it and the database enforces what it may touch."
	>
		{#if loadError}
			<Alert type="error" title="Could not load permissions" size="xs">{loadError}</Alert>
		{:else if loading}
			<span class="text-sm text-tertiary">Loading...</span>
		{:else}
			<div class="flex flex-col gap-4">
				<Toggle
					bind:checked={enabled}
					disabled={isFork && !enabled}
					options={{
						right: 'Enable permissions',
						rightTooltip:
							'While off, every workspace member reaches this data table through its single connection. Turning it off again drops the roles created here, after giving their objects back to admin.'
					}}
				/>

				{#if isFork && !enabled}
					<Alert type="info" title="Permissions belong to the workspace this was forked from" size="xs">
						A fork's data table points either at that workspace's database, where roles created
						here would be invisible to its own configuration, or at a copy this fork can drop.
						Enable permissions there instead, once this fork is deleted.
					</Alert>
				{/if}

				{#if enabled}
					<DataTable>
						<Head>
							<tr>
								<Cell head first>
									Role
									<Tooltip>
										admin is the connection the data table already used, so it owns every existing
										object and cannot be renamed or removed. Every other role gets its own Postgres
										login, created with no privileges — grant it what it needs from admin.
									</Tooltip>
								</Cell>
								<Cell head>
									Tenants
									<Tooltip>
										Users, groups and folders allowed to run as this role. Workspace admins can use
										every role.
									</Tooltip>
								</Cell>
								<Cell head>
									Default
									<Tooltip>
										The role a script gets when it names none — no `-- role` annotation, no `?role=`
										in an ATTACH. Callers still have to be one of its tenants.
									</Tooltip>
								</Cell>
								<Cell head last />
							</tr>
						</Head>
						<tbody class="divide-y bg-surface-tertiary">
							{#each roles as role (role.id)}
								{@const isRoot = role.name === ADMIN_DATATABLE_ROLE}
								<Row>
									<Cell first class="w-56 align-top">
										<div class="flex flex-col gap-1">
											<TextInput
												bind:value={role.name}
												inputProps={{ placeholder: 'Role name', disabled: isRoot }}
											/>
											{#if role.pg_rolename}
												<span class="text-2xs text-tertiary font-mono select-all"
													>{role.pg_rolename}</span
												>
											{/if}
										</div>
									</Cell>
									<Cell class="align-top">
										<MultiSelect
											items={tenantItems.current ?? []}
											bind:value={role.tenants}
											groupBy={(item) => item.group}
											placeholder="Nobody — Add users, groups or folders"
										/>
									</Cell>
									<Cell class="w-20 align-top">
										<div class="flex justify-center pt-2">
											<Checkbox
												checked={defaultRoleId === role.id}
												title="Use this role when a script names none"
												onChange={() => (defaultRoleId = role.id)}
											/>
										</div>
									</Cell>
									<Cell last class="w-10 align-top">
										{#if !isRoot}
											<CloseButton small on:close={() => removeRole(role.id)} />
										{/if}
									</Cell>
								</Row>
							{/each}
							<Row class="!border-0">
								<Cell colspan={4} class="pt-0 pb-2">
									<div class="flex justify-center">
										<Button
											unifiedSize="sm"
											btnClasses="max-w-fit"
											variant="default"
											on:click={addRole}
										>
											<Plus /> New role
										</Button>
									</div>
								</Cell>
							</Row>
						</tbody>
					</DataTable>
					{#if nameError}
						<span class="text-xs text-red-600">{nameError}</span>
					{/if}
				{/if}
			</div>
		{/if}

		{#if enabled && !hasUnsavedChanges && !loadError && !loading}
			<div class="mt-6 pt-6 border-t">
				<PgAclEditor {workspace} {datatable} target={{ kind: 'database' }} showOwner={false} />
			</div>
		{/if}

		{#snippet actions()}
			<Button
				variant="accent"
				unifiedSize="md"
				disabled={!hasUnsavedChanges || loading || !!loadError || !!nameError}
				loading={saving}
				on:click={requestPreview}
			>
				Save
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<!-- Portalled to the body: this button is also mounted inside the database
	manager's drawer, whose own stacking context would otherwise trap the modal
	underneath the permissions drawer sitting next to it. -->
<Portal>
	<ConfirmationModal
		open={!!preview}
		title="Apply permission changes"
		confirmationText={preview?.statements.length ? 'Run and save' : 'Save'}
		type="info"
		loading={applying}
		onConfirmed={apply}
		onCanceled={() => (preview = undefined)}
	>
		<div class="flex flex-col gap-3 min-w-0">
			{#each preview?.warnings ?? [] as warning}
				<Alert type="warning" title="Warning" size="xs">{warning}</Alert>
			{/each}
			{#if !preview?.statements.length}
				<span class="text-sm text-secondary">
					No SQL to run — only the tenants of existing roles changed.
				</span>
			{:else}
				<span class="text-sm text-secondary">
					The following runs against <span class="font-mono">{datatable}</span> in a single transaction:
				</span>
				<pre class="overflow-auto text-xs bg-surface-secondary p-3 rounded select-all max-h-80"
					>{preview.statements.join('\n')}</pre
				>
			{/if}
		</div>
	</ConfirmationModal>
</Portal>
