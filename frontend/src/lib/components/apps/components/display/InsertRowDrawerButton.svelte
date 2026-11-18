<script lang="ts">
	import { Alert, Drawer, DrawerContent, Button, ButtonType } from '$lib/components/common'
	import { CornerDownLeft, Plus } from 'lucide-svelte'
	import InsertRow from './dbtable/InsertRow.svelte'
	import type { ColumnDef } from './dbtable/utils'
	import type { DbType } from '$lib/components/dbTypes'
	import { fieldsBlamedByInsertError } from './dbtable/insertRowError'

	type Props = {
		columnDefs: ColumnDef[]
		dbType: DbType
		/** Must reject when the insert fails: the drawer then stays open with the entered values. */
		onInsert: (args: Record<string, any>) => void | Promise<void>
		/** Sizes the trigger on the unified scale; without it the trigger keeps its legacy size. */
		unifiedSize?: ButtonType.UnifiedSize
	}

	let { columnDefs, dbType, onInsert, unifiedSize }: Props = $props()

	let args: Record<string, any> = $state({})
	let insertDrawer: Drawer | undefined = $state()
	let drawerOpen = $state(false)
	let isInsertable = $state(false)
	let inserting = $state(false)
	let insertError: { message: string; fields: string[] } | undefined = $state()

	const onConfirm = async () => {
		if (inserting || !isInsertable) return
		inserting = true
		insertError = undefined
		try {
			await onInsert(args)
		} catch (e) {
			const message = (e instanceof Error ? e.message : String(e)) || 'Insert failed'
			insertError = {
				message,
				fields: fieldsBlamedByInsertError(
					message,
					columnDefs.map((c) => ({ name: c.field, datatype: c.datatype ?? '' })),
					args
				)
			}
			return
		} finally {
			inserting = false
		}
		insertDrawer?.closeDrawer()
		args = {}
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (drawerOpen && (e.ctrlKey || e.metaKey) && e.key === 'Enter') {
			onConfirm()
		}
	}}
/>

<Button
	startIcon={{ icon: Plus }}
	variant="default"
	size={unifiedSize ? undefined : 'xs2'}
	{unifiedSize}
	on:click={() => {
		insertDrawer?.openDrawer()
	}}
>
	Insert
</Button>

<Drawer bind:this={insertDrawer} bind:open={drawerOpen} size="800px">
	<DrawerContent title="Insert row" on:close={insertDrawer.closeDrawer} id="insert-row-drawer">
		{#snippet actions()}
			<Button
				variant="accent"
				unifiedSize="md"
				on:click={onConfirm}
				disabled={!isInsertable}
				loading={inserting}
				shortCut={{ Icon: CornerDownLeft }}
			>
				Insert
			</Button>
		{/snippet}
		{#if insertError}
			<Alert type="error" title="Insert failed" class="mb-4">
				<span class="whitespace-pre-wrap break-words">{insertError.message}</span>
				{#if insertError.fields.length > 0}
					<div class="mt-1">
						Check {insertError.fields.length > 1 ? 'fields' : 'field'}
						<b>{insertError.fields.join(', ')}</b>
					</div>
				{/if}
			</Alert>
		{/if}
		<InsertRow
			bind:args
			bind:isInsertable
			{columnDefs}
			{dbType}
			blamedFields={insertError?.fields ?? []}
		/>
	</DrawerContent>
</Drawer>
