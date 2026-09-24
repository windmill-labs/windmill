<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SqlRepl from './SqlRepl.svelte'
	import DBTable from './DBTable.svelte'
	import type { DbInput } from './dbTypes'
	import { staticDbTableOps } from './staticDbTableOps'
	import { getDbType } from './dbOps'

	type Props = {
		input: DbInput
		workspace: string
		tag?: string
		placeholderTableName?: string
		initialCode?: string
		onCodeChange?: (code: string) => void
		onSchemaChange?: () => void
	}
	let {
		input,
		workspace,
		tag,
		placeholderTableName,
		initialCode,
		onCodeChange,
		onSchemaChange
	}: Props = $props()

	let result: Record<string, unknown>[] | undefined = $state.raw()
	let resultOps = $derived(
		result ? staticDbTableOps(result, getDbType(input), 'query_result') : undefined
	)
</script>

<Splitpanes horizontal>
	<Pane class="relative" minSize={15}>
		<SqlRepl
			{input}
			{workspace}
			{tag}
			{placeholderTableName}
			{initialCode}
			{onCodeChange}
			{onSchemaChange}
			onData={(data) => (result = data)}
		/>
	</Pane>
	<Pane size={55} minSize={10} class="relative">
		{#if resultOps}
			<!-- Each result has its own columns: start its grid afresh. -->
			{#key resultOps}
				<DBTable dbTableOps={resultOps} />
			{/key}
		{:else}
			<div class="h-full w-full center-center bg-surface-tertiary text-xs text-hint">
				Run a query to see its result here
			</div>
		{/if}
	</Pane>
</Splitpanes>
