<script lang="ts">
	import { Button } from './common'
	import Modal2 from './common/modal/Modal2.svelte'
	import NewDataTableMigrationModal from './workspaceSettings/NewDataTableMigrationModal.svelte'
	import { splitSqlStatements, isDdlStatement } from './sqlDdl'

	let { workspace, datatable }: { workspace: string; datatable: string } = $props()

	type Choice = 'run' | 'migrate' | 'cancel'

	let promptStatement = $state<string | undefined>(undefined)
	let promptOpen = $state(false)
	let resolvePrompt: ((choice: Choice) => void) | undefined = undefined
	let resolveMigrationClosed: (() => void) | undefined = undefined
	let newMigrationModal = $state<NewDataTableMigrationModal | undefined>(undefined)

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

	function promptDdl(statement: string): Promise<Choice> {
		return new Promise((resolve) => {
			resolvePrompt = resolve
			promptStatement = statement
			promptOpen = true
		})
	}

	function handleMigrationClosed() {
		const r = resolveMigrationClosed
		resolveMigrationClosed = undefined
		r?.()
	}

	// Open the prefilled new-migration modal and resolve once it closes (whether
	// the migration was created or the user cancelled).
	function openMigrationModal(statement: string): Promise<void> {
		return new Promise((resolve) => {
			resolveMigrationClosed = resolve
			newMigrationModal?.open({ codeUp: statement })
		})
	}

	/**
	 * Inspect `code` for DDL statements. For each one, prompt the user to run it
	 * anyway or turn it into a migration (prompts shown one at a time). Returns
	 * whether to proceed and the code to run (with migrated statements stripped).
	 */
	export async function guard(code: string): Promise<{ proceed: boolean; code: string }> {
		const statements = splitSqlStatements(code)
		if (!statements.some((s) => isDdlStatement(s))) {
			return { proceed: true, code }
		}

		const kept: string[] = []
		for (const statement of statements) {
			if (!isDdlStatement(statement)) {
				kept.push(statement)
				continue
			}
			const choice = await promptDdl(statement)
			if (choice === 'cancel') {
				return { proceed: false, code }
			}
			if (choice === 'run') {
				kept.push(statement)
				continue
			}
			// migrate: strip from the executed code and open the prefilled modal
			await openMigrationModal(statement)
		}

		return { proceed: true, code: kept.join(';\n') }
	}
</script>

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
			<Button variant="accent" size="sm" on:click={() => finishPrompt('migrate')}>
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
/>
