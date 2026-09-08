<script lang="ts">
	import { Alert, Badge, Button } from '../common'
	import CloseButton from '../common/CloseButton.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import Cell from '../table/Cell.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import { Plus } from 'lucide-svelte'
	import { SettingService, type InstanceDatatableRole } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	let roles = $state<InstanceDatatableRole[]>([])
	let loading = $state(true)
	let loadError = $state<string | undefined>(undefined)
	let busy = $state(false)
	let newName = $state('')
	/** Which role's name is being edited, and to what. */
	let renaming = $state<{ id: string; name: string } | undefined>(undefined)

	const confirmationModal = createAsyncConfirmationModal()

	async function load() {
		loading = true
		loadError = undefined
		try {
			roles = await SettingService.listInstanceDatatableRoles()
		} catch (e) {
			loadError = e?.body ?? e?.message ?? String(e)
		} finally {
			loading = false
		}
	}
	load()

	async function run(fn: () => Promise<unknown>, success: string) {
		busy = true
		try {
			await fn()
			sendUserToast(success)
			await load()
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			busy = false
		}
	}

	async function create() {
		const name = newName.trim()
		if (!name) return
		await run(
			() =>
				SettingService.createInstanceDatatableRole({
					requestBody: { name }
				}),
			`Created the data table role ${name}`
		)
		newName = ''
	}

	async function remove(role: InstanceDatatableRole) {
		const confirmed = await confirmationModal.ask({
			title: `Delete the role ${role.name}?`,
			children:
				'Everything it owns in every instance database is handed back to the admin connection, its grants are dropped, and it is removed from every data table that named it. This cannot be undone.',
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
	<div class="flex items-baseline gap-1">
		<h3 class="font-semibold text-sm">Instance roles</h3>
		<Tooltip>
			A data table role is a real Postgres login on this instance, shared by every instance
			database. A job that names one connects as it, and Postgres decides what it may touch — grant
			it privileges with SQL. Which people may use a role on a given data table is set per data
			table, in its roles drawer.
		</Tooltip>
	</div>

	{#if loadError}
		<Alert type="error" title="Could not load the instance roles" size="xs">{loadError}</Alert>
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
										disabled={busy}
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
								<button
									class="font-mono text-sm hover:underline"
									onclick={() => (renaming = { id: role.id, name: role.name })}
								>
									{role.name}
								</button>
							{/if}
						</Cell>
						<Cell>
							<div class="flex items-center gap-2">
								<Toggle
									checked={role.enabled}
									disabled={busy}
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
