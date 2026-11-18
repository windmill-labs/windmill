<script lang="ts">
	import { Alert, Badge, Button } from '../common'
	import CloseButton from '../common/CloseButton.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Toggle from '../Toggle.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import Cell from '../table/Cell.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import { Pencil, Plus } from 'lucide-svelte'
	import { SettingService, type DatatableRoleCluster, type InstanceDatatableRole } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { handleRoleCreationError } from './datatableRoleModals'

	let {
		initialName = '',
		pinnedCluster,
		onChanged
	}: {
		/** Prefills the name of the role to add. */
		initialName?: string
		/** The one catalog to manage, when the caller has a cluster of its own: a data table can
		 *  only grant the roles of the cluster it sits on, so its drawer pins that one. Left out,
		 *  the section offers whichever clusters this instance has. */
		pinnedCluster?: DatatableRoleCluster
		/** Called after every change to the catalog, whether or not it went through. */
		onChanged?: () => void
	} = $props()

	let cluster = $state<DatatableRoleCluster>(pinnedCluster ?? 'instance')
	/** The databases a role here reaches, for the copy: a drop is explained by what it undoes. */
	let clusterName = $derived(
		cluster === 'external_instance' ? 'the external cluster' : "Windmill's database"
	)
	/** Whether the external cluster is configured, so its catalog is worth offering. Only a
	 *  superadmin can read that, and only a superadmin manages roles. */
	let externalConfigured = $state(false)
	/** Windmill's database turned off as a data table substrate, and whether roles are still defined
	 *  on it. Its data tables stop resolving while it is off, but their databases and grants remain,
	 *  so while any role is left its catalog stays reachable to clean it up or to turn it back on. */
	let internalTurnedOff = $state(false)
	let internalHasRoles = $state(false)
	let internalAvailable = $derived(!isCloudHosted() && (!internalTurnedOff || internalHasRoles))
	/** Neither catalog can be offered, so nothing here may create a role on either cluster. */
	let clusterAvailable = $derived(
		pinnedCluster !== undefined ||
			(cluster === 'external_instance' ? externalConfigured : internalAvailable)
	)
	let roles = $state<InstanceDatatableRole[]>([])
	let loading = $state(true)
	let loadError = $state<string | undefined>(undefined)
	let busy = $state(false)
	// svelte-ignore state_referenced_locally
	let newName = $state(initialName)
	/** Which role's name is being edited, and to what. */
	let renaming = $state<{ id: string; name: string } | undefined>(undefined)

	const confirmationModal = createAsyncConfirmationModal()

	/** Identifies the load in flight. A switch back and forth leaves two requests racing, and the
	 *  slower one must not seat another cluster's roles under the selected one: a name exists on
	 *  both clusters, so the rows would look right while every control acted on the wrong id. */
	let loadSeq = 0

	/** Resolves to the roles it seated, or undefined when it failed or was overtaken. */
	async function load(): Promise<InstanceDatatableRole[] | undefined> {
		const seq = ++loadSeq
		loading = true
		loadError = undefined
		try {
			const fresh = await SettingService.listInstanceDatatableRoles({ cluster })
			if (seq !== loadSeq) return
			roles = fresh
			return fresh
		} catch (e) {
			if (seq !== loadSeq) return
			loadError = e?.body ?? e?.message ?? String(e)
		} finally {
			if (seq === loadSeq) loading = false
		}
	}
	const initialLoad = load()

	if (pinnedCluster === undefined) {
		Promise.all([
			SettingService.getExternalInstancePgStatus()
				.then((s) => s.configured)
				.catch(() => false),
			SettingService.getGlobal({ key: 'instance_pg_disabled' })
				.then((v) => !!v)
				.catch(() => false),
			// Unpinned, the first load is Windmill's database. One it could not read counts as having
			// roles, so the catalog is not hidden on a guess.
			initialLoad
		]).then(([external, turnedOff, internalRoles]) => {
			externalConfigured = external
			internalTurnedOff = turnedOff
			internalHasRoles = internalRoles === undefined || internalRoles.length > 0
			// Opened on Windmill's database by default; land on the cluster that is actually in use.
			if (!internalAvailable && externalConfigured) switchCluster('external_instance')
		})
	}

	async function switchCluster(next: DatatableRoleCluster) {
		if (next === cluster) return
		cluster = next
		// The old cluster's rows go with it: leaving them on screen offers controls that would act
		// on the catalog no longer selected.
		roles = []
		renaming = undefined
		await load()
	}

	async function run(fn: () => Promise<unknown>, success: string) {
		busy = true
		try {
			await fn()
			sendUserToast(success)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			// Reloaded whether or not it worked: the login toggle is driven by what the server
			// holds, so a failed flip has to snap back rather than sit there claiming it landed.
			await load()
			busy = false
			onChanged?.()
		}
	}

	async function create() {
		const name = newName.trim()
		if (!name) return
		busy = true
		try {
			await SettingService.createInstanceDatatableRole({ requestBody: { name, cluster } })
			sendUserToast(`Created the data table role ${name}`)
			newName = ''
		} catch (e) {
			const createdOn = cluster
			handleRoleCreationError(e, {
				name,
				cluster: createdOn,
				confirmationModal,
				onTakenOver: async () => {
					newName = ''
					if (cluster === createdOn) await load()
					onChanged?.()
				}
			})
		} finally {
			await load()
			busy = false
			onChanged?.()
		}
	}

	async function remove(role: InstanceDatatableRole) {
		const confirmed = await confirmationModal.ask({
			title: `Delete the role ${role.name}?`,
			children: `Everything it owns in every database Windmill manages on ${clusterName} is handed back to the admin connection, its grants are dropped, and it is removed from every data table that named it. This cannot be undone.`,
			confirmationText: 'Delete role'
		})
		if (!confirmed) return
		await run(
			() => SettingService.deleteInstanceDatatableRole({ id: role.id }),
			`Deleted the data table role ${role.name}`
		)
	}
</script>

<ConfirmationModal {...confirmationModal.props} />

<div class="flex flex-col gap-2">
	{#if pinnedCluster === undefined}
		<!-- Each cluster keeps its own logins, so the catalogs are separate lists, not one
		filtered view. Offered only where a caller has not pinned one. -->
		<ToggleButtonGroup
			bind:selected={() => cluster, (v) => switchCluster(v)}
			class="w-fit"
			disabled={busy}
		>
			{#snippet children({ item })}
				<ToggleButton
					value="instance"
					label="Windmill's database"
					disabled={!internalAvailable}
					tooltip={internalAvailable
						? undefined
						: isCloudHosted()
							? "Windmill's database is not available on cloud."
							: "Windmill's database is turned off for data tables, and has no roles left."}
					{item}
					small
				/>
				<ToggleButton
					value="external_instance"
					label="External cluster"
					disabled={!externalConfigured}
					tooltip={externalConfigured
						? undefined
						: 'No external cluster is set up. Configure one under Managed Postgres.'}
					{item}
					small
				/>
			{/snippet}
		</ToggleButtonGroup>
	{/if}
	{#if !clusterAvailable}
		<Alert type="info" title="No Postgres to define roles on" size="xs">
			Windmill's database is turned off for data tables and no external cluster is set up. Configure
			one under Instance settings → Managed Postgres to manage its roles here.
		</Alert>
	{:else if loadError}
		<Alert type="error" title="Could not load the data table roles" size="xs">{loadError}</Alert>
	{:else}
		<DataTable>
			<Head>
				<tr>
					<Cell head first>Name</Cell>
					<Cell head>Login</Cell>
					<Cell head last></Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface-tertiary">
				{#if loading}
					<Row>
						<Cell colspan={3} class="text-center py-4 text-secondary text-xs">Loading…</Cell>
					</Row>
				{:else if roles.length === 0}
					<Row>
						<Cell colspan={3} class="text-center py-4 text-secondary text-xs">
							No data table role yet. Every job connects as
							<span class="font-mono">admin</span>.
						</Cell>
					</Row>
				{/if}
				{#each roles as role (role.id)}
					<Row>
						<Cell first class="w-64">
							{#if renaming?.id === role.id}
								<div class="flex gap-1 items-center">
									<TextInput bind:value={renaming.name} inputProps={{ placeholder: 'Name' }} />
									<Button
										unifiedSize="xs"
										variant="accent"
										disabled={busy || loading}
										on:click={async () => {
											const name = renaming?.name?.trim()
											renaming = undefined
											if (name && name !== role.name) {
												await run(
													() =>
														SettingService.updateInstanceDatatableRole({
															id: role.id,
															requestBody: { name }
														}),
													`Renamed the data table role to ${name}`
												)
											}
										}}
									>
										Rename
									</Button>
									<CloseButton small on:close={() => (renaming = undefined)} />
								</div>
							{:else}
								<div class="flex items-center gap-1">
									<span class="font-mono text-sm">{role.name}</span>
									<Button
										unifiedSize="2xs"
										variant="subtle"
										startIcon={{ icon: Pencil }}
										iconOnly
										title="Rename this role"
										on:click={() => (renaming = { id: role.id, name: role.name })}
									/>
								</div>
							{/if}
						</Cell>
						<Cell>
							<div class="flex items-center gap-2">
								<Toggle
									checked={role.enabled}
									disabled={busy || loading}
									on:change={(e) =>
										run(
											() =>
												SettingService.updateInstanceDatatableRole({
													id: role.id,
													requestBody: { enabled: e.detail }
												}),
											e.detail ? `Enabled ${role.name}` : `Disabled ${role.name}`
										)}
								/>
								{#if !role.enabled}
									<Badge color="gray">Cannot log in</Badge>
								{/if}
							</div>
						</Cell>
						<Cell last class="w-12">
							<CloseButton small on:close={() => remove(role)} />
						</Cell>
					</Row>
				{/each}
				<Row class="!border-0">
					<Cell colspan={3} class="pt-0 pb-2">
						<div class="flex gap-2 items-center">
							<TextInput
								bind:value={newName}
								inputProps={{ placeholder: 'analytics', id: 'new-datatable-role' }}
							/>
							<Button
								unifiedSize="sm"
								variant="default"
								startIcon={{ icon: Plus }}
								disabled={busy || !newName.trim()}
								on:click={create}
							>
								Add role
							</Button>
						</div>
					</Cell>
				</Row>
			</tbody>
		</DataTable>
	{/if}
</div>
