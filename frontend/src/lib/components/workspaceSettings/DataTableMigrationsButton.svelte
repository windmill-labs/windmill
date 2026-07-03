<script lang="ts">
	import { Button } from '../common'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import TabContent from '../common/tabs/TabContent.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import {
		ChevronDown,
		Play,
		Trash2,
		Plus,
		Undo2,
		Loader2,
		Camera,
		Settings,
		Power,
		PowerOff
	} from 'lucide-svelte'
	import { WorkspaceService, type DatatableMigrationWithStatus } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import NewDataTableMigrationModal from './NewDataTableMigrationModal.svelte'
	import Tooltip from '../Tooltip.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import { superadmin, userStore } from '$lib/stores'

	let {
		workspace,
		datatable,
		disabled = false
	}: { workspace: string; datatable: string; disabled?: boolean } = $props()

	let listOpen = $state(false)
	let migrations = $state<DatatableMigrationWithStatus[]>([])
	let enabled = $state(true)
	let loadError = $state<string | undefined>(undefined)
	let loading = $state(false)
	let busy = $state(false)

	// Only workspace admins and super admins can opt a data table in or out.
	const canManage = $derived(!!$userStore?.is_admin || !!$superadmin)

	let newMigrationModal = $state<NewDataTableMigrationModal | undefined>(undefined)
	let newMigrationOpen = $state(false)

	let viewOpen = $state(false)
	let viewMigration = $state<DatatableMigrationWithStatus | undefined>(undefined)
	let viewTab = $state('up')
	let settingsDropdownOpen = $state(false)

	function openView(m: DatatableMigrationWithStatus) {
		viewMigration = m
		viewTab = 'up'
		viewOpen = true
	}

	const confirmationModal = createAsyncConfirmationModal()

	const hasPending = $derived(migrations.some((m) => m.status !== 'ran'))

	async function loadMigrations() {
		// Only show the full-list spinner on the initial load. Refreshes after an
		// action (run/revert/delete) update the keyed list in place to avoid a
		// flicker, with the buttons already gated by `busy`.
		if (migrations.length === 0) {
			loading = true
		}
		try {
			const res = await WorkspaceService.getDatatableMigrationsStatus({
				workspace,
				datatableName: datatable
			})
			enabled = res.enabled
			migrations = res.migrations
			loadError = res.error
		} catch (e: any) {
			sendUserToast(`Failed to load migrations: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			loading = false
		}
	}

	function openList() {
		listOpen = true
		loadMigrations()
	}

	async function runUpTo(upTo: number | undefined) {
		busy = true
		try {
			const res = await WorkspaceService.runDatatableMigrations({
				workspace,
				datatableName: datatable,
				upTo
			})
			sendUserToast(
				res.applied.length > 0
					? `Applied ${res.applied.length} migration(s)`
					: 'No pending migrations to run'
			)
			await loadMigrations()
		} catch (e: any) {
			sendUserToast(`Failed to run migrations: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			busy = false
		}
	}

	async function runOnly(version: number) {
		busy = true
		try {
			const res = await WorkspaceService.runDatatableMigrations({
				workspace,
				datatableName: datatable,
				only: version
			})
			sendUserToast(
				res.applied.length > 0 ? `Applied ${res.applied[0].name}` : 'Migration already applied'
			)
			await loadMigrations()
		} catch (e: any) {
			sendUserToast(`Failed to run migration: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			busy = false
		}
	}

	// Run a single migration. Warn first if earlier migrations haven't run, since
	// this one might depend on them.
	async function runMigration(m: DatatableMigrationWithStatus) {
		const earlierPending = migrations.filter(
			(x) => x.timestamp < m.timestamp && x.status !== 'ran'
		).length
		if (earlierPending > 0) {
			const confirmed = await confirmationModal.ask({
				title: 'Run migration out of order',
				confirmationText: 'Run anyway',
				children: `${earlierPending} earlier migration(s) have not been run yet. "${m.name}" might depend on them. Run it anyway?`
			})
			if (!confirmed) return
		}
		await runOnly(m.timestamp)
	}

	async function revertOnly(version: number) {
		busy = true
		try {
			const res = await WorkspaceService.rollbackDatatableMigrations({
				workspace,
				datatableName: datatable,
				only: version
			})
			sendUserToast(
				res.rolled_back.length > 0
					? `Reverted ${res.rolled_back[0].name}`
					: 'Migration was not applied'
			)
			await loadMigrations()
		} catch (e: any) {
			sendUserToast(`Failed to revert migration: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			busy = false
		}
	}

	// Revert a single migration. Warn first if later migrations have run, since
	// they might depend on this one.
	async function revertMigration(m: DatatableMigrationWithStatus) {
		const laterApplied = migrations.filter(
			(x) => x.timestamp > m.timestamp && x.status === 'ran'
		).length
		if (laterApplied > 0) {
			const confirmed = await confirmationModal.ask({
				title: 'Revert migration out of order',
				confirmationText: 'Revert anyway',
				children: `${laterApplied} later migration(s) have already been run and might depend on "${m.name}". Reverting it may break them. Revert anyway?`
			})
			if (!confirmed) return
		}
		await revertOnly(m.timestamp)
	}

	async function generateInitial() {
		const confirmed = await confirmationModal.ask({
			title: 'Generate initial migration',
			confirmationText: 'Generate',
			children: `This snapshots the current schema of "${datatable}" with pg_dump as an "initial" migration and marks it as already applied, so it is never re-run on this data table. It has no down migration. Use this to start tracking migrations on an existing data table.`
		})
		if (!confirmed) return
		busy = true
		try {
			await WorkspaceService.generateInitialDatatableMigration({
				workspace,
				datatableName: datatable
			})
			sendUserToast('Initial migration generated')
			await loadMigrations()
		} catch (e: any) {
			sendUserToast(`Failed to generate initial migration: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			busy = false
		}
	}

	async function enableMigrations() {
		busy = true
		try {
			await WorkspaceService.enableDatatableMigrations({ workspace, datatableName: datatable })
			sendUserToast('Migrations enabled')
			await loadMigrations()
		} catch (e: any) {
			sendUserToast(`Failed to enable migrations: ${e?.body ?? e?.message ?? e}`, true)
			return
		} finally {
			busy = false
		}
		// Right after enabling, offer to snapshot the current schema as the initial migration.
		await generateInitial()
	}

	async function disableMigrations() {
		const confirmed = await confirmationModal.ask({
			title: 'Disable migrations',
			confirmationText: 'Disable and delete',
			children: `Disabling migrations for "${datatable}" will delete ALL of its migrations. This cannot be undone. The data table's data and schema are not affected. Continue?`
		})
		if (!confirmed) return
		busy = true
		try {
			await WorkspaceService.disableDatatableMigrations({ workspace, datatableName: datatable })
			sendUserToast('Migrations disabled')
			await loadMigrations()
		} catch (e: any) {
			sendUserToast(`Failed to disable migrations: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			busy = false
		}
	}

	async function deleteMigration(m: DatatableMigrationWithStatus) {
		const body =
			m.status === 'ran'
				? `"${m.name}" is installed on the data table. Deleting its definition (including its down migration) means it can no longer be reverted and may leave the data table in a broken state. Delete anyway?`
				: m.status === 'unknown'
					? `The applied status of "${m.name}" is unknown. If it is installed on the data table, deleting it may leave the data table in a broken state. Delete anyway?`
					: `Delete the definition of "${m.name}"? It has not been run, so this only removes the migration definition.`
		const confirmed = await confirmationModal.ask({
			title: 'Delete migration',
			confirmationText: 'Delete',
			children: body
		})
		if (!confirmed) return
		busy = true
		try {
			await WorkspaceService.deleteDatatableMigration({
				workspace,
				datatableName: datatable,
				timestamp: m.timestamp
			})
			await loadMigrations()
		} catch (e: any) {
			sendUserToast(`Failed to delete migration: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			busy = false
		}
	}

	const statusColor = {
		ran: 'bg-green-500',
		not_run: 'bg-orange-500',
		unknown: 'bg-gray-400'
	}
	const statusTitle = {
		ran: 'Already ran',
		not_run: 'Not run',
		unknown: 'Status unknown'
	}
</script>

<Button variant="default" size="sm" {disabled} endIcon={{ icon: ChevronDown }} on:click={openList}>
	Migrations
</Button>

<Modal2
	bind:isOpen={listOpen}
	title="Migrations — {datatable}"
	fixedWidth="md"
	fixedHeight="lg"
	closeOnOutsideClick={!newMigrationOpen &&
		!viewOpen &&
		!confirmationModal.props.open &&
		!settingsDropdownOpen}
>
	{#snippet headerLeft()}
		<Tooltip>
			Each schema edit made in the database manager is captured here as a migration. Tracking schema
			changes as migrations makes it easy to export data tables to other workspaces, and to
			reproduce their schema when forking a workspace.
		</Tooltip>
	{/snippet}
	{#snippet headerRight()}
		{#if enabled && canManage}
			<DropdownV2
				bind:open={settingsDropdownOpen}
				items={[
					{
						displayName: 'Disable migrations',
						icon: PowerOff,
						type: 'delete',
						action: () => disableMigrations()
					}
				]}
			>
				{#snippet buttonReplacement()}
					<Button
						variant="subtle"
						size="xs"
						iconOnly
						startIcon={{ icon: Settings }}
						title="Migration settings"
						disabled={busy}
					/>
				{/snippet}
			</DropdownV2>
		{/if}
	{/snippet}
	<div class="flex flex-col gap-2 w-full grow min-h-0">
		{#if loading}
			<div class="flex items-center justify-center grow text-tertiary">
				<Loader2 size={18} class="animate-spin" />
			</div>
		{:else if !enabled}
			<div class="flex flex-col items-center justify-center gap-3 grow text-sm text-tertiary">
				<span>Migrations are disabled for this data table.</span>
				{#if canManage}
					<Button
						variant="subtle"
						size="sm"
						startIcon={{ icon: Power }}
						disabled={busy}
						on:click={enableMigrations}
					>
						Enable migrations
					</Button>
				{/if}
			</div>
		{:else}
			{#if loadError}
				<div class="text-xs text-red-500">
					Could not read applied status from the data table: {loadError}
				</div>
			{/if}
			<div class="flex flex-col grow min-h-0 overflow-auto border rounded-md divide-y">
				{#if migrations.length === 0}
					<div class="flex flex-col items-center gap-3 p-6 text-sm text-tertiary">
						<span>No migrations yet</span>
						<Button
							variant="subtle"
							size="xs"
							startIcon={{ icon: Camera }}
							disabled={busy}
							on:click={generateInitial}
						>
							Generate initial migration
						</Button>
					</div>
				{:else}
					{#each migrations as m (m.timestamp)}
						<div class="flex items-center gap-3 px-3 py-2">
							<div
								class="shrink-0 w-2.5 h-2.5 rounded-full {statusColor[m.status]}"
								title={statusTitle[m.status]}
							></div>
							<button
								type="button"
								class="flex flex-col min-w-0 grow text-left rounded -mx-1 px-1 py-0.5 hover:bg-surface-hover"
								title="View migration"
								onclick={() => openView(m)}
							>
								<span class="text-sm text-primary truncate">{m.name}</span>
								<span class="text-xs text-hint">{m.timestamp}</span>
							</button>
							<Button
								variant="subtle"
								size="xs"
								iconOnly
								startIcon={{ icon: Play }}
								title="Run this migration"
								disabled={busy || m.status === 'ran'}
								on:click={() => runMigration(m)}
							/>
							<Button
								variant="subtle"
								size="xs"
								iconOnly
								startIcon={{ icon: Undo2 }}
								title="Revert this migration"
								disabled={busy || m.status !== 'ran'}
								on:click={() => revertMigration(m)}
							/>
							<Button
								variant="subtle"
								size="xs"
								iconOnly
								color="red"
								startIcon={{ icon: Trash2 }}
								title="Delete migration"
								disabled={busy}
								on:click={() => deleteMigration(m)}
							/>
						</div>
					{/each}
				{/if}
			</div>
			<div class="flex justify-between gap-2 pt-2">
				<Button
					variant="default"
					size="sm"
					startIcon={{ icon: Plus }}
					on:click={() => {
						newMigrationOpen = true
						newMigrationModal?.open()
					}}>New</Button
				>
				<Button
					variant="accent"
					size="sm"
					startIcon={{ icon: Play }}
					disabled={busy || !hasPending}
					on:click={() => runUpTo(undefined)}
				>
					Run all
				</Button>
			</div>
		{/if}
	</div>
</Modal2>

<NewDataTableMigrationModal
	bind:this={newMigrationModal}
	{workspace}
	{datatable}
	onCreated={loadMigrations}
	onClose={() => (newMigrationOpen = false)}
/>

<Modal2
	bind:isOpen={viewOpen}
	title="Migration — {viewMigration?.name ?? ''}"
	fixedWidth="md"
	fixedHeight="lg"
>
	{#if viewMigration}
		<div class="flex flex-col gap-3 w-full grow min-h-0">
			<span class="text-xs text-hint">{viewMigration.timestamp}</span>
			<Tabs bind:selected={viewTab} class="grow min-h-0">
				<Tab value="up" label="Up" />
				<Tab value="down" label="Down" />
				{#snippet content()}
					<TabContent value="up" class="h-80 border rounded-md overflow-hidden">
						<SimpleEditor class="h-full" lang="sql" code={viewMigration?.code_up ?? ''} readOnly />
					</TabContent>
					<TabContent value="down" class="h-80 border rounded-md overflow-hidden">
						{#if viewMigration?.code_down}
							<SimpleEditor class="h-full" lang="sql" code={viewMigration.code_down} readOnly />
						{:else}
							<div class="p-6 text-center text-sm text-tertiary">No down migration</div>
						{/if}
					</TabContent>
				{/snippet}
			</Tabs>
		</div>
	{/if}
</Modal2>

<Portal>
	<ConfirmationModal {...confirmationModal.props} />
</Portal>
