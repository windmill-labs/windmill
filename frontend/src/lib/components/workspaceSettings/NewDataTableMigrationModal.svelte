<script lang="ts">
	import { Button } from '../common'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import TabContent from '../common/tabs/TabContent.svelte'
	import Toggle from '../Toggle.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { tick } from 'svelte'

	let {
		workspace,
		datatable,
		onCreated,
		onClose
	}: {
		workspace: string
		datatable: string
		/** Called after a successful create; `ran` is true when it was also run. */
		onCreated?: (ran: boolean) => void
		/** Called whenever the modal closes. `result` reports whether the close
		 * was due to a create (and whether that create was also run) vs a cancel —
		 * computed synchronously so callers don't depend on onCreated/onClose order. */
		onClose?: (result: { created: boolean; ran: boolean }) => void
	} = $props()

	let isOpen = $state(false)
	// The reason the modal is about to close, set synchronously before `isOpen`
	// flips so the onClose effect reports it reliably regardless of effect timing.
	let closeResult = { created: false, ran: false }
	let prevOpen = false
	$effect(() => {
		if (prevOpen && !isOpen) {
			onClose?.(closeResult)
			closeResult = { created: false, ran: false }
		}
		prevOpen = isOpen
	})
	let tab = $state('up')
	let name = $state('')
	let nameInput = $state<TextInput>()
	let codeUp = $state('')
	let enableDown = $state(false)
	let codeDown = $state('')
	let creating = $state(false)

	// Frame the migration body in an explicit transaction so it applies atomically.
	function wrapInTransaction(body: string): string {
		return `BEGIN;\n\n${body}\n\nEND;`
	}
	// A statement inside BEGIN; ... END; must be `;`-terminated. The SQL splitter
	// strips the trailing `;` when extracting a detected DDL statement, so re-add
	// it when missing.
	function ensureTrailingSemicolon(body: string): string {
		const trimmed = body.trimEnd()
		return trimmed.endsWith(';') ? trimmed : `${trimmed};`
	}
	const PLACEHOLDER = wrapInTransaction('-- Add your migration here')

	export function open(prefill?: { name?: string; codeUp?: string; codeDown?: string }) {
		name = prefill?.name ?? ''
		// Start from the transaction template; when prefilled from detected DDL,
		// wrap that DDL in the same BEGIN; ... END; frame.
		codeUp = prefill?.codeUp
			? wrapInTransaction(ensureTrailingSemicolon(prefill.codeUp))
			: PLACEHOLDER
		codeDown = prefill?.codeDown ?? PLACEHOLDER
		enableDown = (prefill?.codeDown ?? '') !== ''
		tab = 'up'
		isOpen = true
		// Focus the name field once the modal content has rendered.
		tick().then(() => nameInput?.focus())
	}

	async function create(run: boolean) {
		if (name.trim() === '') {
			sendUserToast('Migration name is required', true)
			return
		}
		if (!/^[a-zA-Z0-9_-]+$/.test(name.trim())) {
			sendUserToast("Invalid migration name: use only letters, digits, '_' and '-'", true)
			return
		}
		creating = true
		try {
			const created = await WorkspaceService.createDatatableMigration({
				workspace,
				datatableName: datatable,
				requestBody: {
					name: name.trim(),
					code_up: codeUp,
					code_down: enableDown ? codeDown : undefined
				}
			})
			if (run) {
				try {
					await WorkspaceService.runDatatableMigrations({
						workspace,
						datatableName: datatable,
						only: created.timestamp
					})
				} catch (runErr: any) {
					// The migration was created but failed to run; undo the insertion so
					// the user can fix the SQL and retry from a clean state.
					await WorkspaceService.deleteDatatableMigration({
						workspace,
						datatableName: datatable,
						timestamp: created.timestamp
					}).catch(() => {})
					sendUserToast(
						`Migration failed to run and was reverted: ${runErr?.body ?? runErr?.message ?? runErr}`,
						true
					)
					return
				}
			}
			closeResult = { created: true, ran: run }
			onCreated?.(run)
			sendUserToast(run ? 'Migration created and run' : 'Migration created')
			isOpen = false
		} catch (e: any) {
			sendUserToast(`Failed to create migration: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			creating = false
		}
	}
</script>

<!-- closeOnOutsideClick is off: the "Create and run" split-button menu renders in a
	portal outside the modal, so an outside-click close would fire on its items and be
	mistaken for a cancel. Close via the header X or Escape instead. -->
<Modal2
	bind:isOpen
	title="New migration — {datatable}"
	fixedWidth="md"
	fixedHeight="adaptive"
	closeOnOutsideClick={false}
>
	<div class="flex flex-col gap-3 w-full grow min-h-0">
		<TextInput
			bind:this={nameInput}
			bind:value={name}
			error={name.trim() === ''}
			inputProps={{ placeholder: 'Migration name (e.g. add_index_to_customers)' }}
		/>
		<Tabs bind:selected={tab} class="grow min-h-0">
			<Tab value="up" label="Up" />
			<Tab value="down" label="Down" />
			{#snippet content()}
				<TabContent value="up" class="h-80 border rounded-md overflow-hidden">
					<SimpleEditor class="h-full" lang="sql" bind:code={codeUp} />
				</TabContent>
				<TabContent value="down" class="h-80">
					<div class="flex flex-col gap-2 h-full">
						<Toggle
							bind:checked={enableDown}
							options={{ right: 'Enable down migration' }}
							size="sm"
						/>
						{#if enableDown}
							<div class="grow min-h-0 border rounded-md overflow-hidden">
								<SimpleEditor class="h-full" lang="sql" bind:code={codeDown} />
							</div>
						{/if}
					</div>
				</TabContent>
			{/snippet}
		</Tabs>
		<div class="flex justify-end pt-2">
			<Button
				variant="accent"
				size="sm"
				disabled={creating}
				on:click={() => create(true)}
				dropdownItems={[
					{
						label: 'Create without running',
						onClick: () => create(false)
					}
				]}
			>
				Create and run
			</Button>
		</div>
	</div>
</Modal2>
