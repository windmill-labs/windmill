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

	let {
		workspace,
		datatable,
		onCreated,
		onClose
	}: {
		workspace: string
		datatable: string
		onCreated?: () => void
		onClose?: () => void
	} = $props()

	let isOpen = $state(false)
	// Notify the parent whenever the modal closes (after a create or a cancel),
	// so callers that sequence modals (e.g. the DDL guard) can advance.
	let prevOpen = false
	$effect(() => {
		if (prevOpen && !isOpen) onClose?.()
		prevOpen = isOpen
	})
	let tab = $state('up')
	let name = $state('')
	let codeUp = $state('')
	let enableDown = $state(false)
	let codeDown = $state('')
	let creating = $state(false)

	export function open(prefill?: { name?: string; codeUp?: string; codeDown?: string }) {
		name = prefill?.name ?? ''
		codeUp = prefill?.codeUp ?? ''
		codeDown = prefill?.codeDown ?? ''
		enableDown = (prefill?.codeDown ?? '') !== ''
		tab = 'up'
		isOpen = true
	}

	async function create(run: boolean) {
		if (name.trim() === '') {
			sendUserToast('Migration name is required', true)
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
			isOpen = false
			sendUserToast(run ? 'Migration created and run' : 'Migration created')
			onCreated?.()
		} catch (e: any) {
			sendUserToast(`Failed to create migration: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			creating = false
		}
	}
</script>

<Modal2 bind:isOpen title="New migration — {datatable}" fixedWidth="md" fixedHeight="adaptive">
	<div class="flex flex-col gap-3 w-full grow min-h-0">
		<TextInput
			bind:value={name}
			inputProps={{ placeholder: 'Migration name (e.g. add_index_to_customers)' }}
		/>
		<Tabs bind:selected={tab} class="grow min-h-0">
			<Tab value="up" label="Up" />
			<Tab value="down" label="Down" />
			{#snippet content()}
				<TabContent value="up" class="h-80">
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
							<div class="grow min-h-0">
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
