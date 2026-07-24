<script lang="ts">
	import { Plus, Shield, Trash } from 'lucide-svelte'
	import { Alert, Button, Drawer } from '../common'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Toggle from '../Toggle.svelte'
	import Select from '../select/Select.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Tooltip from '../Tooltip.svelte'
	import {
		FolderService,
		GroupService,
		UserService,
		WorkspaceService,
		type DataTableGrant,
		type DataTablePermissions
	} from '$lib/gen'
	import { enterpriseLicense, superadmin, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { deepEqual } from 'fast-equals'

	let {
		workspace,
		datatable,
		disabled = false,
		permissionsEnabled = false
	}: {
		workspace: string
		datatable: string
		disabled?: boolean
		/// Whether the data table currently has permissions enabled: keeps the
		/// button visible without an EE license so admins can still disable the
		/// feature after a license downgrade (non-admin access fails closed).
		permissionsEnabled?: boolean
	} = $props()

	const OPERATIONS = ['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE'] as const

	// Admin only; EE-gated except as an escape hatch when already enabled.
	const visible = $derived(
		(!!$enterpriseLicense || permissionsEnabled) && (!!$userStore?.is_admin || !!$superadmin)
	)

	let drawer: Drawer | undefined = $state()
	let loading = $state(false)
	let saving = $state(false)
	let perms: DataTablePermissions | undefined = $state(undefined)
	let savedPerms: DataTablePermissions | undefined = $state(undefined)
	let usernames: string[] = $state([])
	let groupNames: string[] = $state([])
	let folderNames: string[] = $state([])
	let schemaTables: Record<string, string[]> = $state({})
	let isExternalDb = $state(false)
	let preflightError: string | undefined = $state(undefined)

	const dirty = $derived(!deepEqual(perms, savedPerms))

	function normalize(p: DataTablePermissions): DataTablePermissions {
		return {
			enabled: p.enabled ?? false,
			grants: (p.grants ?? []).map((g) => ({
				tenant: g.tenant,
				folder_access: g.folder_access,
				operations: g.operations ?? [],
				schema: g.schema,
				tables: g.tables ?? []
			}))
		}
	}

	async function openDrawer() {
		drawer?.openDrawer()
		loading = true
		preflightError = undefined
		try {
			const [p, users, groups, folders, tables, datatables] = await Promise.all([
				WorkspaceService.getDataTablePermissions({ workspace, datatableName: datatable }),
				UserService.listUsernames({ workspace }),
				GroupService.listGroupNames({ workspace }),
				FolderService.listFolderNames({ workspace }),
				WorkspaceService.listDataTableTables({ workspace }),
				WorkspaceService.listDataTables({ workspace })
			])
			perms = normalize(p)
			savedPerms = normalize(p)
			usernames = users
			groupNames = groups
			folderNames = folders
			schemaTables = tables.find((t) => t.datatable_name === datatable)?.schemas ?? {}
			isExternalDb = datatables.find((d) => d.name === datatable)?.resource_type !== 'instance'
		} catch (e) {
			sendUserToast(e, true)
		} finally {
			loading = false
		}
	}

	async function onToggleEnabled(enabled: boolean) {
		preflightError = undefined
		if (!enabled || !perms) return
		try {
			await WorkspaceService.checkDataTablePermissionsSetup({
				workspace,
				datatableName: datatable
			})
		} catch (e: any) {
			preflightError = e?.body ?? e?.message ?? String(e)
		}
	}

	function tenantKind(tenant: string): 'user' | 'group' | 'folder' {
		if (tenant.startsWith('g/')) return 'group'
		if (tenant.startsWith('f/')) return 'folder'
		return 'user'
	}

	function tenantName(tenant: string): string {
		const idx = tenant.indexOf('/')
		return idx >= 0 ? tenant.slice(idx + 1) : tenant
	}

	function setTenantKind(grant: DataTableGrant, kind: 'user' | 'group' | 'folder') {
		grant.tenant = { user: 'u/', group: 'g/', folder: 'f/' }[kind]
		grant.folder_access = kind === 'folder' ? 'read' : undefined
	}

	function tenantItems(kind: 'user' | 'group' | 'folder') {
		const names = kind === 'user' ? usernames : kind === 'group' ? groupNames : folderNames
		return safeSelectItems(names)
	}

	function addGrant() {
		if (!perms) return
		perms.grants = [
			...(perms.grants ?? []),
			{
				tenant: 'u/',
				operations: ['SELECT'],
				schema: Object.keys(schemaTables)[0] ?? 'public',
				tables: []
			}
		]
	}

	function removeGrant(index: number) {
		if (!perms) return
		perms.grants = (perms.grants ?? []).filter((_, i) => i !== index)
	}

	async function onSave() {
		if (!perms) return
		for (const grant of perms.grants ?? []) {
			if (!tenantName(grant.tenant)) {
				sendUserToast('Every grant must have a user, group or folder selected', true)
				return
			}
			if ((grant.operations ?? []).length === 0) {
				sendUserToast('Every grant must have at least one operation', true)
				return
			}
			if (!grant.schema) {
				sendUserToast('Every grant must target a schema', true)
				return
			}
		}
		saving = true
		try {
			await WorkspaceService.setDataTablePermissions({
				workspace,
				datatableName: datatable,
				requestBody: perms
			})
			savedPerms = normalize($state.snapshot(perms))
			sendUserToast('Data table permissions saved')
		} catch (e) {
			sendUserToast(e, true)
		} finally {
			saving = false
		}
	}
</script>

{#if visible}
	<Button variant="default" size="sm" {disabled} startIcon={{ icon: Shield }} on:click={openDrawer}>
		Permissions
	</Button>

	<Drawer bind:this={drawer} size="900px">
		<DrawerContent title="Permissions for data table {datatable}" on:close={drawer?.closeDrawer}>
			{#snippet actions()}
				<Button variant="accent" size="sm" disabled={!dirty || loading} {saving} on:click={onSave}>
					Save permissions
				</Button>
			{/snippet}
			{#if loading || !perms}
				<div class="text-sm text-secondary p-4">Loading...</div>
			{:else}
				<div class="flex flex-col gap-6">
					<div class="flex flex-col gap-2">
						<Toggle
							bind:checked={
								() => perms?.enabled ?? false,
								(v) => {
									if (perms) {
										perms.enabled = v
										onToggleEnabled(v)
									}
								}
							}
							options={{ right: 'Enable fine-grained permissions' }}
						/>
						<p class="text-xs text-secondary">
							When enabled, users without a matching grant lose all access to this data table
							(default deny), including from scripts and flows they run. Workspace admins and
							superadmins are unaffected and keep full access. Access is enforced by PostgreSQL
							itself through short-lived per-user roles.
						</p>
						{#if preflightError}
							<Alert type="error" title="Cannot enable permissions" size="xs">
								{preflightError}
							</Alert>
						{/if}
					</div>

					<div class="flex flex-col gap-3">
						<div class="flex items-center justify-between">
							<span class="text-sm font-semibold text-emphasis">
								Grants ({perms.grants?.length ?? 0})
								<Tooltip>
									Each grant gives a user, a group, or the users of a folder a set of SQL operations
									on a schema or on specific tables. A user's effective access is the union of all
									matching grants.
								</Tooltip>
							</span>
							<Button size="sm" variant="default" on:click={addGrant}>
								<Plus size={14} /> Add grant
							</Button>
						</div>
						{#if (perms.grants ?? []).length === 0}
							<div class="text-xs text-tertiary italic">
								No grants: only admins can access this data table while permissions are enabled.
							</div>
						{/if}
						{#each perms.grants ?? [] as grant, i (i)}
							<div class="flex flex-col gap-2 p-3 border rounded bg-surface-secondary">
								<div class="flex flex-row flex-wrap gap-2 items-center">
									<ToggleButtonGroup
										selected={tenantKind(grant.tenant)}
										on:selected={(e) => setTenantKind(grant, e.detail)}
									>
										{#snippet children({ item })}
											<ToggleButton value="user" small label="User" {item} />
											<ToggleButton value="group" small label="Group" {item} />
											<ToggleButton value="folder" small label="Folder" {item} />
										{/snippet}
									</ToggleButtonGroup>
									{#key tenantKind(grant.tenant)}
										<Select
											items={tenantItems(tenantKind(grant.tenant))}
											bind:value={
												() => tenantName(grant.tenant),
												(name) => {
													grant.tenant = grant.tenant.slice(0, 2) + (name ?? '')
												}
											}
											placeholder="Select {tenantKind(grant.tenant)}"
											class="grow min-w-40"
										/>
									{/key}
									{#if tenantKind(grant.tenant) === 'folder'}
										<ToggleButtonGroup
											selected={grant.folder_access ?? 'read'}
											on:selected={(e) => (grant.folder_access = e.detail)}
										>
											{#snippet children({ item })}
												<ToggleButton value="read" small label="Folder readers" {item} />
												<ToggleButton value="write" small label="Folder writers" {item} />
											{/snippet}
										</ToggleButtonGroup>
										<Tooltip>
											The grant applies to users having at least this access level on the folder.
										</Tooltip>
									{/if}
									<div class="grow"></div>
									<Button
										variant="default"
										destructive
										size="xs"
										iconOnly
										startIcon={{ icon: Trash }}
										on:click={() => removeGrant(i)}
									/>
								</div>
								<div class="flex flex-row flex-wrap gap-2 items-center">
									<MultiSelect
										items={OPERATIONS.map((o) => ({ label: o, value: o }))}
										bind:value={grant.operations}
										placeholder="Operations"
										class="min-w-56"
										size="sm"
									/>
									<Select
										items={safeSelectItems(Object.keys(schemaTables))}
										bind:value={grant.schema}
										placeholder="Schema"
										class="w-36"
									/>
									<MultiSelect
										items={safeSelectItems(schemaTables[grant.schema] ?? [])}
										bind:value={() => grant.tables ?? [], (v) => (grant.tables = v)}
										placeholder="All tables (incl. future ones)"
										class="grow min-w-56"
										size="sm"
									/>
								</div>
							</div>
						{/each}
					</div>

					{#if isExternalDb}
						<Alert type="warning" title="External database" size="xs">
							Enabling permissions makes Windmill create and drop PostgreSQL roles on the fly
							<span class="font-semibold">on your external database server</span> (its user needs the
							CREATEROLE privilege). These short-lived roles can also connect to other databases on that
							server that still grant CONNECT to PUBLIC (the PostgreSQL default) — revoke it there if
							that matters to you.
						</Alert>
					{/if}

					<Alert type="info" title="How enforcement works" size="xs">
						<ul class="list-disc ml-4 flex flex-col gap-1 mt-1">
							<li>
								Grants are enforced by PostgreSQL through short-lived per-user roles; schema usage
								and sequence usage (for INSERT/UPDATE) are granted automatically.
							</li>
							<li>
								A grant with no table selected covers the whole schema including tables created
								later; table-specific grants only cover the listed tables.
							</li>
							<li>
								DDL (CREATE/ALTER/DROP) is never grantable: schema changes go through migrations,
								which become admin-only when permissions are enabled.
							</li>
							<li>
								Changes to grants, groups or folders take effect on the next access; a connection
								already open keeps its privileges for up to 5 minutes.
							</li>
							<li>
								Creating or editing Postgres triggers on this data table, and running its
								migrations, require workspace admin while permissions are enabled.
							</li>
						</ul>
					</Alert>
				</div>
			{/if}
		</DrawerContent>
	</Drawer>
{/if}
