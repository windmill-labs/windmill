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
	import { KeyRound } from 'lucide-svelte'
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
	import InstanceRolesButton from './InstanceRolesButton.svelte'

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
	// lists are keyed by — so renaming a role instance-side moves nothing here. A row without an id
	// names a role the instance does not define yet: it cannot be saved until a superadmin creates
	// it, and takes the new role's id once they have.
	type EditedRole = { id: string | undefined; name: string | undefined; tenants: string[] }
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
	// The instance catalog, read again after the instance roles drawer changes it.
	let catalog = $state<InstanceDatatableRole[] | undefined>(undefined)
	const availableRoles: InstanceDatatableRole[] = $derived(catalog ?? info?.available_roles ?? [])
	const unusedRoles = $derived(availableRoles.filter((r) => !roles.some((row) => row.id === r.id)))
	const pendingRoles = $derived(roles.filter((r) => r.id === undefined))
	let instanceRoles: InstanceRolesButton | undefined = $state(undefined)

	const roleKey = (role: EditedRole) => role.id ?? `pending:${role.name}`

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

	/** Adds a role by name: the instance's role of that name, or a pending row for one it does not
	 * define yet. */
	function addRoleByName(typed: string) {
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
		roles.push({ id: existing?.id, name, tenants: [] })
	}

	function removeRole(role: EditedRole) {
		const key = roleKey(role)
		roles = roles.filter((r) => roleKey(r) !== key)
		if (role.id !== undefined && defaultRoleId === role.id) defaultRoleId = ADMIN_DATATABLE_ROLE
	}

	/** Reads the instance catalog again and gives each pending row the id of the role now defined
	 * under its name. */
	async function refreshCatalog() {
		let fresh: InstanceDatatableRole[]
		try {
			fresh = await SettingService.listInstanceDatatableRoles()
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
			return
		}
		catalog = fresh
		for (const row of roles) {
			if (row.id !== undefined) continue
			const created = fresh.find((r) => r.name === row.name)
			if (created) row.id = created.id
		}
	}

	async function save() {
		saving = true
		try {
			const msg = await WorkspaceService.setDatatablePermissions({
				workspace,
				datatableName: datatable,
				requestBody: {
					permissioned,
					default_role: defaultRoleId,
					roles: roles.map((r) => ({ id: r.id!, tenants: $state.snapshot(r.tenants) }))
				}
			})
			sendUserToast(msg)
			await load()
			onSaved?.()
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			saving = false
		}
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

<Drawer bind:open={drawerOpen} size="900px">
	<DrawerContent
		title="Roles — {datatable}"
		on:close={() => (drawerOpen = false)}
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
					{#if editable && availableRoles.length === 0}
						<Alert type="warning" title="No role defined on this instance" size="xs">
							Only <span class="font-mono">admin</span> can be used until a superadmin creates a data
							table role. Type a name below to add one.
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
							{#each roles as role (roleKey(role))}
								{@const isAdmin = role.id === ADMIN_DATATABLE_ROLE}
								<Row>
									<Cell first class="w-56 align-top">
										<div class="flex flex-col gap-0.5 pt-1.5">
											<span class="font-mono text-xs text-emphasis">{role.name ?? role.id}</span>
											{#if !role.name}
												<span class="text-2xs text-secondary italic">
													no longer defined on this instance
												</span>
											{:else if role.id === undefined}
												<Alert type="warning" title="This role does not exist yet" size="xs">
													{#if $superadmin}
														<div class="flex flex-col items-start gap-1">
															<span>Create it on the instance to use it here.</span>
															<Button
																unifiedSize="xs"
																variant="default"
																on:click={() => instanceRoles?.open(role.name)}
															>
																Create it
															</Button>
														</div>
													{:else}
														Only a superadmin can create it on the instance.
													{/if}
												</Alert>
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
												checked={role.id !== undefined && defaultRoleId === role.id}
												disabled={!editable || role.id === undefined}
												title="Use this role when a job names none"
												onChange={() => {
													if (role.id !== undefined) defaultRoleId = role.id
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
								<Row class="!border-0">
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

			{#if info?.supported && !hasUnsavedChanges}
				<div class="mt-6 pt-6 border-t">
					<PgAclEditor {workspace} {datatable} target={{ kind: 'database' }} />
				</div>
			{/if}
		{/if}

		{#snippet actions()}
			{#if editable}
				<Button
					variant="accent"
					unifiedSize="md"
					disabled={!hasUnsavedChanges || loading || !!loadError || pendingRoles.length > 0}
					title={pendingRoles.length > 0
						? 'Create the roles that do not exist yet, or remove them'
						: undefined}
					loading={saving}
					on:click={save}
				>
					Save
				</Button>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>

{#if $superadmin}
	<InstanceRolesButton bind:this={instanceRoles} hideTrigger onChanged={refreshCatalog} />
{/if}
