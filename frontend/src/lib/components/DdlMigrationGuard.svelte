<script lang="ts">
	import { Button } from './common'
	import Modal2 from './common/modal/Modal2.svelte'
	import NewDataTableMigrationModal from './workspaceSettings/NewDataTableMigrationModal.svelte'
	import DataTableMigrationsButton from './workspaceSettings/DataTableMigrationsButton.svelte'
	import { splitSqlStatements, isDdlStatement } from './sqlDdl'
	import { CornerDownLeft } from 'lucide-svelte'

	let { workspace, datatable }: { workspace: string; datatable: string } = $props()

	type Choice = 'run' | 'migrate' | 'cancel'

	let promptStatement = $state<string | undefined>(undefined)
	let promptOpen = $state(false)
	let resolvePrompt: ((choice: Choice) => void) | undefined = undefined
	let resolveMigrationClosed: ((created: boolean) => void) | undefined = undefined
	// Set when a migration is created *and run* during a guard() call, so the
	// caller can refresh the schema afterwards.
	let migrationRan = false
	let newMigrationModal = $state<NewDataTableMigrationModal | undefined>(undefined)
	// Triggerless Migrations modal, opened programmatically from the "See migration"
	// toast action after a migration is created here.
	let migrationsModal = $state<DataTableMigrationsButton | undefined>(undefined)

	function finishPrompt(choice: Choice) {
		const r = resolvePrompt
		resolvePrompt = undefined
		promptOpen = false
		promptStatement = undefined
		r?.(choice)
	}

	// Closing the modal (X / escape) while a prompt is pending counts as cancel.
	$effect(() => {
		if (!promptOpen && resolvePrompt) {
			finishPrompt('cancel')
		}
	})

	// Enter confirms the primary action ("Create a migration"). Ignore modified
	// Enter so editor shortcuts (Cmd/Ctrl+Enter) keep working underneath.
	function onKeyDown(event: KeyboardEvent) {
		if (!promptOpen || event.metaKey || event.ctrlKey || event.altKey) return
		if (event.key === 'Enter') {
			event.stopPropagation()
			event.preventDefault()
			finishPrompt('migrate')
		}
	}

	function promptDdl(statement: string): Promise<Choice> {
		return new Promise((resolve) => {
			resolvePrompt = resolve
			promptStatement = statement
			promptOpen = true
		})
	}

	function handleMigrationClosed(result: { created: boolean; ran: boolean }) {
		if (result.ran) migrationRan = true
		const r = resolveMigrationClosed
		resolveMigrationClosed = undefined
		r?.(result.created)
	}

	// Open the prefilled new-migration modal. Resolves with whether a migration
	// was actually created (false if the user cancelled / closed it).
	function openMigrationModal(statement: string): Promise<boolean> {
		return new Promise((resolve) => {
			resolveMigrationClosed = (created: boolean) => resolve(created)
			newMigrationModal?.open({ codeUp: statement })
		})
	}

	/**
	 * Inspect `code` for DDL statements. For each one, prompt the user to run it
	 * anyway or turn it into a migration (prompts shown one at a time). Returns
	 * whether to proceed and the code to run (with migrated statements stripped).
	 */
	export async function guard(
		code: string
	): Promise<{ proceed: boolean; code: string; ranMigration: boolean }> {
		migrationRan = false
		const statements = splitSqlStatements(code)
		if (!statements.some((s) => isDdlStatement(s))) {
			return { proceed: true, code, ranMigration: false }
		}

		const kept: string[] = []
		for (const statement of statements) {
			if (!isDdlStatement(statement)) {
				kept.push(statement)
				continue
			}
			// Re-prompt for this statement until the user makes a terminal choice;
			// cancelling the migration modal returns to the prompt with the DDL intact.
			for (;;) {
				const choice = await promptDdl(statement)
				if (choice === 'cancel') {
					return { proceed: false, code, ranMigration: migrationRan }
				}
				if (choice === 'run') {
					kept.push(statement)
					break
				}
				// migrate: only strip the statement once a migration is actually
				// created; if the modal was cancelled, loop back to the prompt.
				const created = await openMigrationModal(statement)
				if (created) {
					break
				}
			}
		}

		return { proceed: true, code: kept.join(';\n'), ranMigration: migrationRan }
	}
</script>

<svelte:window onkeydown={onKeyDown} />

<Modal2
	title="Schema change detected"
	fixedWidth="md"
	fixedHeight="adaptive"
	bind:isOpen={promptOpen}
	closeOnOutsideClick={false}
>
	<div class="flex flex-col gap-3 w-full">
		<p class="text-sm text-secondary">
			This looks like a schema-changing (DDL) statement. Schema changes are best tracked as
			migrations rather than run ad-hoc. Create a migration for it instead?
		</p>
		<pre
			class="text-xs whitespace-pre-wrap font-mono bg-surface-secondary rounded p-3 max-h-48 overflow-auto"
			>{promptStatement ?? ''}</pre
		>
		<div class="flex justify-end gap-2 pt-2">
			<Button variant="default" size="sm" on:click={() => finishPrompt('run')}>Run anyway</Button>
			<Button
				variant="accent"
				size="sm"
				shortCut={{ Icon: CornerDownLeft, withoutModifier: true }}
				on:click={() => finishPrompt('migrate')}
			>
				Create a migration
			</Button>
		</div>
	</div>
</Modal2>

<NewDataTableMigrationModal
	bind:this={newMigrationModal}
	{workspace}
	{datatable}
	onClose={handleMigrationClosed}
	onSeeMigration={(m) => migrationsModal?.openMigration(m.timestamp)}
/>

<DataTableMigrationsButton bind:this={migrationsModal} hideTrigger {workspace} {datatable} />
