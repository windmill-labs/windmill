<script lang="ts">
	import { Button } from '../common'
	import Modal2 from '../common/modal/Modal2.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import { ChevronDown, Play, Trash2, Plus, Undo2, Loader2 } from 'lucide-svelte'
	import { WorkspaceService, type DatatableMigrationWithStatus } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import NewDataTableMigrationModal from './NewDataTableMigrationModal.svelte'

	let {
		workspace,
		datatable,
		disabled = false
	}: { workspace: string; datatable: string; disabled?: boolean } = $props()

	let listOpen = $state(false)
	let migrations = $state<DatatableMigrationWithStatus[]>([])
	let loadError = $state<string | undefined>(undefined)
	let loading = $state(false)
	let busy = $state(false)

	let newMigrationModal = $state<NewDataTableMigrationModal | undefined>(undefined)

	const confirmationModal = createAsyncConfirmationModal()

	const hasPending = $derived(migrations.some((m) => m.status !== 'ran'))
	const hasApplied = $derived(migrations.some((m) => m.status === 'ran'))

	async function loadMigrations() {
		loading = true
		try {
			const res = await WorkspaceService.getDatatableMigrationsStatus({
				workspace,
				datatableName: datatable
			})
			migrations = res.migrations
			loadError = res.error
		} catch (e) {
			sendUserToast(`Failed to load migrations: ${e}`, true)
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
		} catch (e) {
			sendUserToast(`Failed to run migrations: ${e}`, true)
		} finally {
			busy = false
		}
	}

	async function revertLast() {
		busy = true
		try {
			const res = await WorkspaceService.rollbackDatatableMigrations({
				workspace,
				datatableName: datatable
			})
			sendUserToast(
				res.rolled_back.length > 0
					? `Rolled back ${res.rolled_back[0].name}`
					: 'No applied migrations to roll back'
			)
			await loadMigrations()
		} catch (e) {
			sendUserToast(`Failed to revert migration: ${e}`, true)
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
		} catch (e) {
			sendUserToast(`Failed to delete migration: ${e}`, true)
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

<Button
	variant="default"
	size="sm"
	{disabled}
	startIcon={{ icon: ChevronDown }}
	on:click={openList}
>
	Migrations
</Button>

<Modal2 bind:isOpen={listOpen} title="Migrations — {datatable}" fixedWidth="md" fixedHeight="lg">
	<div class="flex flex-col gap-2 w-full grow min-h-0">
		{#if loadError}
			<div class="text-xs text-red-500">
				Could not read applied status from the data table: {loadError}
			</div>
		{/if}
		<div class="flex flex-col grow min-h-0 overflow-auto border rounded-md divide-y">
			{#if loading}
				<div class="flex items-center justify-center p-6 text-tertiary">
					<Loader2 size={18} class="animate-spin" />
				</div>
			{:else if migrations.length === 0}
				<div class="p-6 text-center text-sm text-tertiary">No migrations yet</div>
			{:else}
				{#each migrations as m (m.timestamp)}
					<div class="flex items-center gap-3 px-3 py-2">
						<div
							class="shrink-0 w-2.5 h-2.5 rounded-full {statusColor[m.status]}"
							title={statusTitle[m.status]}
						></div>
						<div class="flex flex-col min-w-0 grow">
							<span class="text-sm text-primary truncate">{m.name}</span>
							<span class="text-xs text-hint">{m.timestamp}</span>
						</div>
						<Button
							variant="subtle"
							size="xs"
							iconOnly
							startIcon={{ icon: Play }}
							title="Run up to this migration"
							disabled={busy || m.status === 'ran'}
							on:click={() => runUpTo(m.timestamp)}
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
				on:click={() => newMigrationModal?.open()}>New</Button
			>
			<div class="flex gap-2">
				<Button
					variant="default"
					size="sm"
					startIcon={{ icon: Undo2 }}
					disabled={busy || !hasApplied}
					on:click={revertLast}
				>
					Revert last
				</Button>
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
		</div>
	</div>
</Modal2>

<NewDataTableMigrationModal
	bind:this={newMigrationModal}
	{workspace}
	{datatable}
	onCreated={loadMigrations}
/>

<ConfirmationModal {...confirmationModal.props} />
