<script lang="ts">
	import { Alert, Badge, Button, Drawer, DrawerContent } from '../common'
	import Select from '../select/Select.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import { KeyRound, Plus } from 'lucide-svelte'
	import {
		FolderService,
		GroupService,
		UserService,
		WorkspaceService,
		type DatatablePermissions,
		type InstanceDatatableRole
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	const ADMIN_ROLE = 'admin'

	let {
		workspace,
		datatable,
		disabled = false
	}: {
		workspace: string
		datatable: string
		disabled?: boolean
	} = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let loading = $state(false)
	let saving = $state(false)
	let loadError = $state<string | undefined>(undefined)
	let info = $state<DatatablePermissions | undefined>(undefined)

	// Edited copy. `id` is the instance role's catalog id (or the reserved `admin`), which is what
	// the tenant lists are keyed by — so renaming a role instance-side moves nothing here.
	let permissioned = $state(false)
	let defaultRole = $state(ADMIN_ROLE)
	let rows = $state<{ id: string; name: string | undefined; tenants: string[] }[]>([])

	// Tenants name principals of the workspace that governs the data table, which is not
	// necessarily the one we are browsing from.
	let tenantOptions = $state<{ value: string; label: string }[]>([])

	const editable = $derived(!!info?.editable)
	const governing = $derived(info?.governing_workspace_id)
	const availableRoles: InstanceDatatableRole[] = $derived(info?.available_roles ?? [])
	const unusedRoles = $derived(availableRoles.filter((r) => !rows.some((row) => row.id === r.id)))

	async function load() {
		loading = true
		loadError = undefined
		try {
			const res = await WorkspaceService.getDatatablePermissions({ workspace, datatableName: datatable })
			info = res
			permissioned = res.permissioned
			defaultRole = res.default_role
			rows = (res.roles ?? [])
				.map((r) => ({ id: r.id, name: r.name, tenants: r.tenants ?? [] }))
				.sort((a, b) => (a.id === ADMIN_ROLE ? -1 : b.id === ADMIN_ROLE ? 1 : 0))
			if (rows.length === 0) {
				rows = [{ id: ADMIN_ROLE, name: ADMIN_ROLE, tenants: [] }]
			}
			await loadTenantOptions(res.governing_workspace_id ?? workspace)
		} catch (e) {
			loadError = e?.body ?? e?.message ?? String(e)
		} finally {
			loading = false
		}
	}

	async function loadTenantOptions(ws: string) {
		try {
			const [users, groups, folders] = await Promise.all([
				UserService.listUsernames({ workspace: ws }),
				GroupService.listGroupNames({ workspace: ws }),
				FolderService.listFolderNames({ workspace: ws })
			])
			tenantOptions = [
				{ value: '*', label: 'Everyone in the workspace' },
				...users.map((u) => ({ value: `u/${u}`, label: `u/${u}` })),
				...groups.map((g) => ({ value: `g/${g}`, label: `g/${g}` })),
				...folders.map((f) => ({ value: `f/${f}`, label: `f/${f}` }))
			]
		} catch {
			// A fork member may not be able to list the governing workspace's principals. The
			// tenants they cannot name are still shown, they just cannot pick new ones.
			tenantOptions = []
		}
	}

	function addRole(id: string) {
		const role = availableRoles.find((r) => r.id === id)
		if (!role) return
		rows = [...rows, { id: role.id, name: role.name, tenants: [] }]
	}

	function removeRole(id: string) {
		rows = rows.filter((r) => r.id !== id)
		if (defaultRole === id) defaultRole = ADMIN_ROLE
	}

	async function save() {
		saving = true
		try {
			const msg = await WorkspaceService.setDatatablePermissions({
				workspace,
				datatableName: datatable,
				requestBody: {
					permissioned,
					default_role: defaultRole,
					roles: rows.map((r) => ({ id: r.id, tenants: r.tenants }))
				}
			})
			sendUserToast(msg)
			await load()
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			saving = false
		}
	}

	export function open() {
		drawer?.openDrawer()
		load()
	}
</script>

<Button
	unifiedSize="sm"
	variant="default"
	startIcon={{ icon: KeyRound }}
	iconOnly
	{disabled}
	title={disabled ? 'Save settings first' : 'Roles: who may connect as which Postgres role'}
	on:click={open}
/>

<Drawer bind:this={drawer} size="700px">
	<DrawerContent
		title="Roles for {datatable}"
		on:close={() => drawer?.closeDrawer()}
		tooltip="A data table role is a Postgres login. A job that names one connects as it, and Postgres decides what it may touch — grant privileges with SQL. Roles are defined for the whole instance; here you say who may use each one on this data table."
	>
		{#if loading}
			<p class="text-sm text-secondary">Loading…</p>
		{:else if loadError}
			<Alert type="error" title="Could not load roles" size="xs">{loadError}</Alert>
		{:else}
			<div class="flex flex-col gap-4">
				{#if governing}
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
							{#each info.ungoverned_reachers as reacher}
								<li>{reacher.workspace_id} / {reacher.datatable}</li>
							{/each}
						</ul>
					</Alert>
				{/if}

				<Toggle
					bind:checked={permissioned}
					disabled={!editable}
					options={{
						right: 'Put this data table under roles',
						rightTooltip:
							'Off, every job connects as admin — the connection that owns every table. On, every job resolves to a role, and a caller no tenant covers is refused.'
					}}
				/>

				{#if permissioned}
					{#if availableRoles.length === 0}
						<Alert type="warning" title="No role defined on this instance" size="xs">
							Only <span class="font-mono">admin</span> can be used until a superadmin adds a data table
							role in the data table settings page.
						</Alert>
					{/if}

					<div class="flex flex-col gap-3">
						{#each rows as row (row.id)}
							<div class="flex flex-col gap-1 border rounded-md p-3 bg-surface-secondary">
								<div class="flex items-center gap-2">
									<Badge color={row.id === ADMIN_ROLE ? 'blue' : 'gray'}>
										{row.name ?? row.id}
									</Badge>
									{#if row.id === ADMIN_ROLE}
										<Tooltip>
											The connection every data table resolved to before roles. It owns every
											existing object, so it is always available and cannot be removed.
										</Tooltip>
									{:else if !row.name}
										<span class="text-xs text-secondary italic">
											no longer defined on this instance
										</span>
									{/if}
									{#if defaultRole === row.id}
										<Badge color="green">Default</Badge>
									{:else if editable}
										<Button
											unifiedSize="2xs"
											variant="subtle"
											on:click={() => (defaultRole = row.id)}
										>
											Make default
										</Button>
									{/if}
									<div class="grow"></div>
									{#if editable && row.id !== ADMIN_ROLE}
										<CloseButton small on:close={() => removeRole(row.id)} />
									{/if}
								</div>
								<MultiSelect
									items={tenantOptions}
									bind:value={row.tenants}
									disabled={!editable}
									placeholder="Nobody yet — add a user, group or folder"
									size="sm"
								/>
							</div>
						{/each}
					</div>

					{#if editable && unusedRoles.length > 0}
						<div class="flex items-center gap-2">
							<Plus size={14} class="text-secondary" />
							<Select
								items={unusedRoles.map((r) => ({
									value: r.id,
									label: r.enabled ? r.name : `${r.name} (disabled)`
								}))}
								placeholder="Add a role"
								bind:value={
									() => undefined,
									(id) => {
										if (id) addRole(id)
									}
								}
								class="w-64"
								size="sm"
							/>
						</div>
					{/if}
				{/if}

				{#if editable}
					<div class="flex justify-end">
						<Button unifiedSize="sm" variant="accent" loading={saving} on:click={save}>
							Save roles
						</Button>
					</div>
				{/if}
			</div>
		{/if}
	</DrawerContent>
</Drawer>
