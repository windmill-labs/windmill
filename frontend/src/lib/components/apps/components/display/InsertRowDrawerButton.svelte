<script lang="ts">
	import { Drawer, DrawerContent, Button, ButtonType } from '$lib/components/common'
	import { CornerDownLeft, Plus } from 'lucide-svelte'
	import InsertRow from './dbtable/InsertRow.svelte'
	import type { ColumnDef } from './dbtable/utils'
	import type { DbType } from '$lib/components/dbTypes'

	type Props = {
		columnDefs: ColumnDef[]
		dbType: DbType
		onInsert: (args: Record<string, any>) => void | Promise<void>
		/** Sizes the trigger on the unified scale; without it the trigger keeps its legacy size. */
		unifiedSize?: ButtonType.UnifiedSize
	}

	let { columnDefs, dbType, onInsert, unifiedSize }: Props = $props()

	let args: Record<string, any> = $state({})
	let insertDrawer: Drawer | undefined = $state()
	let isInsertable = $state(false)

	const onConfirm = async () => {
		await onInsert(args)
		insertDrawer?.closeDrawer()
		args = {}
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
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
		args = {}
		insertDrawer?.openDrawer()
	}}
>
	Insert
</Button>

<Drawer bind:this={insertDrawer} size="800px">
	<DrawerContent title="Insert row" on:close={insertDrawer.closeDrawer} id="insert-row-drawer">
		{#snippet actions()}
			<Button
				variant="accent"
				unifiedSize="md"
				on:click={onConfirm}
				disabled={!isInsertable}
				shortCut={{ Icon: CornerDownLeft }}
			>
				Insert
			</Button>
		{/snippet}
		<InsertRow bind:args bind:isInsertable {columnDefs} {dbType} />
	</DrawerContent>
</Drawer>
