<script lang="ts">
	import { Drawer, DrawerContent, Button } from '$lib/components/common'
	import { Plus } from 'lucide-svelte'
	import InsertRow from './dbtable/InsertRow.svelte'
	import type { ColumnDef, DbType } from './dbtable/utils'

	type Props = {
		columnDefs: ColumnDef[]
		dbType: DbType
		onInsert: (args: Record<string, any>) => void | Promise<void>
	}

	let { columnDefs, dbType, onInsert }: Props = $props()

	let args: Record<string, any> = $state({})
	let insertDrawer: Drawer | undefined = $state()
	let isInsertable = $state(false)
</script>

<Button
	startIcon={{ icon: Plus }}
	color="dark"
	size="xs2"
	on:click={() => {
		args = {}
		insertDrawer?.openDrawer()
	}}
>
	Insert
</Button>

<Drawer bind:this={insertDrawer} size="800px">
	<DrawerContent title="Insert row" on:close={insertDrawer.closeDrawer}>
		<svelte:fragment slot="actions">
			<Button
				color="dark"
				size="xs"
				on:click={async () => {
					await onInsert(args)
					insertDrawer?.closeDrawer()
					args = {}
				}}
				disabled={!isInsertable}>Insert</Button
			>
		</svelte:fragment>
		<InsertRow bind:args bind:isInsertable {columnDefs} {dbType} />
	</DrawerContent>
</Drawer>
