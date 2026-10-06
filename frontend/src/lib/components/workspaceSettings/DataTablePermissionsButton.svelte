<script lang="ts">
	import { Alert, Badge, Button, Drawer, DrawerContent } from '../common'
	import Select from '../select/Select.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import Checkbox from '../common/checkbox/Checkbox.svelte'
	import Cell from '../table/Cell.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import { CircleHelp, KeyRound } from 'lucide-svelte'
	import {
		FolderService,
		GroupService,
		SettingService,
		UserService,
		WorkspaceService,
		type DatatablePermissions,
		type InstanceDatatableRole
	} from '$lib/gen'
	import { superadmin } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { deepEqual } from 'fast-equals'
	import { ADMIN_DATATABLE_ROLE, isDatatableRoleName } from '../dbTypes'
	import PgAclEditor from '../datatableAcl/PgAclEditor.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import {
		pgInstanceName as pgInstanceNameOf,
		handleRoleCreationError,
		offerUnusedRoleDrop
	} from './datatableRoleModals'

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
		/** Mount the drawer without its button, for a caller that opens it with `open()`. */
		hideTrigger?: boolean
		/** Called once a save went through, so a caller showing the roles can read them again. */
		onSaved?: () => void
	} = $props()

	// `id` is the instance role's catalog id (or the reserved `admin`), which is what the tenant
	// lists are keyed by — so renaming a role instance-side moves nothing here.
	type EditedRole = { id: string; name: string | undefined; tenants: string[] }
	type Edited = { permissioned: boolean; roles: EditedRole[]; defaultRoleId: string }

	let drawerOpen = $state(false)
	let loading = $state(false)
	let saving = $state(false)
	let loadError = $state<string | undefined>(undefined)
	let info = $state<DatatablePermissions | undefined>(undefined)

	let permissioned = $state(false)
	let roles = $state<EditedRole[]>([])
	let defaultRoleId = $state(ADMIN_DATATABLE_ROLE)
	/** The last loaded state, to detect unsaved changes against. */
	let saved = $state<Edited>({
		permissioned: false,
		roles: [],
		defaultRoleId: ADMIN_DATATABLE_ROLE
	})

	// Tenants name principals of the workspace that governs the data table, which is not
	// necessarily the one we are browsing from.
	let tenantItems = $state<{ value: string; label: string; group: string }[]>([])

	const editable = $derived(!!info?.editable)
	const governing = $derived(info?.governing_workspace_id)
	// The instance catalog, with the roles created from this drawer since it was loaded.
	let catalog = $state<InstanceDatatableRole[] | undefined>(undefined)
	const availableRoles: InstanceDatatableRole[] = $derived(catalog ?? info?.available_roles ?? [])
	// A role is a login on one cluster, and this drawer's is the data table's: name it, or the
	// copy would send someone to the wrong catalog.
	const clusterName = $derived(
		info?.cluster === 'external_instance' ? 'the external cluster' : "Windmill's database"
	)
	const pgInstanceName = $derived(pgInstanceNameOf(info?.cluster))
	const unusedRoles = $derived(availableRoles.filter((r) => !roles.some((row) => row.id === r.id)))
	const confirmationModal = createAsyncConfirmationModal()
	let aclEditor = $state<PgAclEditor | undefined>(undefined)

	const hasUnsavedChanges = $derived(
		!deepEqual($state.snapshot(saved), {
			permissioned,
			roles: $state.snapshot(roles) as EditedRole[],
			defaultRoleId
		})
	)

	async function load() {
		loading = true
		loadError = undefined
		try {
			const res = await WorkspaceService.getDatatablePermissions({
				workspace,
				datatableName: datatable
			})
			const loaded: EditedRole[] = res.roles
				.map((r) => ({ id: r.id, name: r.name, tenants: [...(r.tenants ?? [])] }))
				.sort(
					(a, b) => Number(b.id === ADMIN_DATATABLE_ROLE) - Number(a.id === ADMIN_DATATABLE_ROLE)
				)
			// A data table never put under roles comes back with none; admin is what turning the toggle
			// on starts from.
			if (!loaded.some((r) => r.id === ADMIN_DATATABLE_ROLE)) {
				loaded.unshift({ id: ADMIN_DATATABLE_ROLE, name: ADMIN_DATATABLE_ROLE, tenants: [] })
			}
			info = res
			catalog = undefined
			permissioned = res.permissioned
			roles = loaded
			defaultRoleId = res.default_role
			saved = structuredClone({ permissioned, roles: loaded, defaultRoleId })
			await loadTenantItems(res.governing_workspace_id ?? workspace)
		} catch (e) {
			loadError = e?.body ?? e?.message ?? String(e)
		} finally {
			loading = false
		}
	}

	async function loadTenantItems(ws: string) {
		try {
			const [users, groups, folders] = await Promise.all([
				UserService.listUsernames({ workspace: ws }),
				GroupService.listGroupNames({ workspace: ws }),
				FolderService.listFolderNames({ workspace: ws })
			])
			tenantItems = [
				{ value: '*', label: 'Everyone', group: 'Anyone in the workspace' },
				...users.map((u) => ({ value: `u/${u}`, label: u, group: 'Users' })),
				...groups.map((g) => ({ value: `g/${g}`, label: g, group: 'Groups' })),
				...folders.map((f) => ({ value: `f/${f}`, label: f, group: 'Folders' }))
			]
		} catch {
			// A fork member may not be able to list the governing workspace's principals. The
			// tenants they cannot name are still shown, they just cannot pick new ones.
			tenantItems = []
		}
	}

	function addRole(id: string) {
		const role = availableRoles.find((r) => r.id === id)
		if (!role) return
		roles.push({ id: role.id, name: role.name, tenants: [] })
	}

	/** Adds the instance's role of that name. One the instance does not define is created first,
	 * which only a superadmin may do. */
	async function addRoleByName(typed: string) {
		const name = typed.trim()
		if (!isDatatableRoleName(name) || name.toLowerCase() === ADMIN_DATATABLE_ROLE) {
			sendUserToast(
				`'${name}' cannot be a data table role name: use letters, digits, '_' and '-'`,
				true
			)
			return
		}
		if (roles.some((r) => r.name === name)) return
		const existing = availableRoles.find((r) => r.name === name)
		if (existing) {
			roles.push({ id: existing.id, name, tenants: [] })
			return
		}
		if (!$superadmin) {
			await confirmationModal.ask({
				title: `Role ${name} does not exist`,
				children: `This role doesn't exist in ${pgInstanceName}. Ask a superadmin to create it.`,
				confirmationText: 'OK',
				type: 'info',
				hideCancel: true
			})
			return
		}
		let created: InstanceDatatableRole | undefined
		let failure: unknown
		await confirmationModal.ask({
			title: `Role ${name} does not exist`,
			children: `(Superadmin) Create this role in ${pgInstanceName}?`,
			confirmationText: 'Create role',
			type: 'info',
			onConfirmed: async () => {
				try {
					created = await SettingService.createInstanceDatatableRole({
						requestBody: { name, cluster: info?.cluster }
					})
				} catch (e) {
					failure = e
				}
			}
		})
		if (created) addCreatedRole(created)
		// Handled once this modal has closed: the take-over offer opens in the same one.
		if (failure) {
			await handleRoleCreationError(failure, {
				name,
				cluster: info?.cluster,
				confirmationModal,
				onTakenOver: addCreatedRole
			})
		}
	}

	function addCreatedRole(created: InstanceDatatableRole) {
		catalog = [...availableRoles, created]
		if (!roles.some((r) => r.id === created.id)) {
			roles.push({ id: created.id, name: created.name, tenants: [] })
		}
	}

	function removeRole(role: EditedRole) {
		roles = roles.filter((r) => r.id !== role.id)
		if (defaultRoleId === role.id) defaultRoleId = ADMIN_DATATABLE_ROLE
	}

	async function save() {
		saving = true
		// Turning roles off clears the whole block, so every role it named is removed.
		const kept = permissioned ? roles.map((r) => r.id) : []
		const removedIds = saved.permissioned
			? saved.roles
					.map((r) => r.id)
					.filter((id) => id !== ADMIN_DATATABLE_ROLE && !kept.includes(id))
			: []
		try {
			const msg = await WorkspaceService.setDatatablePermissions({
				workspace,
				datatableName: datatable,
				requestBody: {
					permissioned,
					default_role: defaultRoleId,
					roles: roles.map((r) => ({ id: r.id, tenants: $state.snapshot(r.tenants) }))
				}
			})
			sendUserToast(msg)
			await load()
			// A role is a login of the whole Postgres instance: whether this was the last data table
			// naming it is only known once the save has landed.
			const unused = availableRoles.filter((r) => removedIds.includes(r.id) && r.in_use === false)
			let dropped = false
			for (const role of unused) {
				dropped =
					(await offerUnusedRoleDrop({
						role,
						cluster: info?.cluster,
						superadmin: !!$superadmin,
						confirmationModal
					})) || dropped
			}
			if (dropped) await load()
			// The grants list the roles they can name, which the save just changed.
			aclEditor?.refresh()
			onSaved?.()
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			saving = false
		}
	}

	/** Whether the drawer may close: unsaved roles are only lost once the user says so. */
	async function confirmDiscard(): Promise<boolean> {
		if (!hasUnsavedChanges || !drawerOpen) return true
		return await confirmationModal.ask({
			title: 'Discard unsaved changes?',
			children: `The roles of ${datatable} have changes that are not saved. Closing the drawer discards them.`,
			confirmationText: 'Discard changes',
			type: 'danger'
		})
	}

	export function open() {
		drawerOpen = true
		load()
	}
</script>

{#if !hideTrigger}
	<Button
		unifiedSize="sm"
		variant="default"
		startIcon={{ icon: KeyRound }}
		iconOnly
		{disabled}
		title={disabled ? 'Save settings first' : 'Roles: who may connect as which Postgres role'}
		on:click={open}
	/>
{/if}

<Drawer bind:open={drawerOpen} size="900px" confirmClose={confirmDiscard}>
	<DrawerContent
		title="Roles: {datatable}"
		on:close={async () => {
			if (await confirmDiscard()) drawerOpen = false
		}}
		tooltip="A data table role is a Postgres login. A job that names one connects as it, and Postgres decides what it may touch — grant it privileges under Access. Roles are defined for the whole instance; here you say who may use each one on this data table."
	>
		{#snippet titleExtra()}
			<Badge color="blue" small>Beta</Badge>
		{/snippet}
		{#if loadError}
			<Alert type="error" title="Could not load roles" size="xs">{loadError}</Alert>
		{:else if loading && !info}
			<span class="text-sm text-secondary">Loading…</span>
		{:else}
			<div class="flex flex-col gap-4">
				<Toggle
					bind:checked={permissioned}
					disabled={!editable || !info?.supported}
					options={{
						right: 'Put this data table under roles',
						rightTooltip:
							'Off, every job connects as admin — the connection that owns every table. On, every job resolves to a role, and a caller no tenant covers is refused.'
					}}
				/>

				{#if !info?.supported}
					<Alert type="info" title="Not available on this data table" size="xs">
						A data table role is a Postgres login on the Windmill instance's own database, so only a
						data table backed by that database can use one. This one is backed by a PostgreSQL
						resource — grant access on that server directly.
					</Alert>
				{:else if info?.clone_of}
					<Alert type="info" title="Governed by {info.clone_of.workspace_id}" size="xs">
						This data table is a clone of <span class="font-mono">{info.clone_of.datatable}</span>
						in workspace <span class="font-mono">{info.clone_of.workspace_id}</span>, so it takes
						its roles from there. You are evaluated as a member of that workspace.
					</Alert>
				{:else if governing}
					<Alert type="info" title="Governed by {governing}" size="xs">
						This data table points at the one in workspace <span class="font-mono">{governing}</span
						>, so its roles are decided there. You are evaluated as a member of that workspace.
					</Alert>
				{:else if !editable}
					<Alert type="info" title="Read only" size="xs">
						Only admins of this workspace can change who may use which role.
					</Alert>
				{/if}

				{#if info?.ungoverned_reachers?.length}
					<Alert type="warning" title="Other workspaces reach this database" size="xs">
						These data tables point at the same database with their own entry, so what you set here
						does not reach them:
						<ul class="mt-1 list-disc list-inside font-mono">
							{#each info.ungoverned_reachers as reacher (`${reacher.workspace_id}/${reacher.datatable}`)}
								<li>{reacher.workspace_id} / {reacher.datatable}</li>
							{/each}
						</ul>
					</Alert>
				{/if}

				{#if permissioned}
					{#if editable && !$superadmin}
						<Alert type="info" title="Only superadmins can create roles" size="xs">
							A data table role is a Postgres login on {pgInstanceName}, so only a superadmin can
							create one. Ask a superadmin for any role you need that is not listed.
							{#if availableRoles.length === 0}
								Until then, only <span class="font-mono">admin</span> can be used.
							{/if}
						</Alert>
					{/if}

					<DataTable>
						<Head>
							<tr>
								<Cell head first>
									Role
									<Tooltip>
										admin is the connection the data table used before roles, so it owns every
										existing object and cannot be removed. Every other role is a login defined for
										the whole instance, with only the privileges granted to it under Access.
									</Tooltip>
								</Cell>
								<Cell head>
									Tenants
									<Tooltip>
										Users, groups and folders allowed to connect as this role. Workspace admins can
										use every role.
									</Tooltip>
								</Cell>
								<Cell head>
									Default
									<Tooltip>
										The role a job gets when it names none — no `-- role` annotation, no `?role=` in
										the reference. Callers still have to be one of its tenants.
									</Tooltip>
								</Cell>
								<Cell head last />
							</tr>
						</Head>
						<tbody class="divide-y bg-surface-tertiary">
							{#each roles as role (role.id)}
								{@const isAdmin = role.id === ADMIN_DATATABLE_ROLE}
								<Row>
									<Cell first class="w-56 align-top">
										<div class="flex flex-col gap-0.5 pt-1.5">
											<span class="font-mono text-xs text-emphasis">{role.name ?? role.id}</span>
											{#if !role.name}
												<span class="text-2xs text-secondary italic">
													no longer defined on {clusterName}
												</span>
											{/if}
										</div>
									</Cell>
									<Cell class="align-top">
										<MultiSelect
											items={tenantItems}
											bind:value={role.tenants}
											groupBy={(item) => item.group}
											disabled={!editable}
											placeholder="Nobody — add users, groups or folders"
										/>
									</Cell>
									<Cell class="w-20 align-top">
										<div class="flex justify-center pt-2">
											<Checkbox
												checked={defaultRoleId === role.id}
												disabled={!editable}
												title="Use this role when a job names none"
												onChange={() => {
													defaultRoleId = role.id
												}}
											/>
										</div>
									</Cell>
									<Cell last class="w-10 align-top">
										{#if editable && !isAdmin}
											<CloseButton small on:close={() => removeRole(role)} />
										{/if}
									</Cell>
								</Row>
							{/each}
							{#if editable}
								<Row>
									<Cell colspan={4} class="pt-2 pb-2">
										<div class="flex justify-center">
											<Select
												items={unusedRoles.map((r) => ({
													value: r.id,
													label: r.enabled ? r.name : `${r.name} (disabled)`
												}))}
												placeholder="+ Add a role"
												bind:value={
													() => undefined,
													(id) => {
														if (id) addRole(id)
													}
												}
												onCreateItem={addRoleByName}
												noItemsMsg={$superadmin ? 'Start typing to create a role' : undefined}
												class="w-64"
											/>
										</div>
									</Cell>
								</Row>
							{/if}
						</tbody>
					</DataTable>
				{/if}
			</div>

			<!-- Grants only matter to a data table under roles: without them every job connects as the
			default login, whatever is granted here. -->
			{#if info?.supported && permissioned}
				<div class="mt-6 pt-6 border-t">
					<PgAclEditor
						bind:this={aclEditor}
						{workspace}
						{datatable}
						target={{ kind: 'database' }}
						disabledReason={hasUnsavedChanges ? 'Save the new roles to continue' : undefined}
					/>
					<div class="mt-3">
						<Button
							unifiedSize="xs"
							variant="subtle"
							startIcon={{ icon: CircleHelp }}
							on:click={() =>
								confirmationModal.ask({
									title: 'Permissions on schemas and tables',
									children: `<p>In the database manager, click the <b>⋮</b> menu of a schema or a table, then <b>Access</b>, to add grants on it or change its owner.</p>`,
									confirmationText: 'OK',
									type: 'info',
									hideCancel: true
								})}
						>
							How to use permissions on schemas and tables?
						</Button>
					</div>
				</div>
			{/if}
		{/if}

		{#snippet actions()}
			{#if editable}
				<Button
					variant="accent"
					unifiedSize="md"
					disabled={!hasUnsavedChanges || loading || !!loadError}
					loading={saving}
					on:click={save}
				>
					Save
				</Button>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>

<ConfirmationModal {...confirmationModal.props} />
