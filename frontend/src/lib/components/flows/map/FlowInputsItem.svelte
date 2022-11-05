<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import { faPen } from '@fortawesome/free-solid-svg-icons'

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<FlowModuleSchemaItem
	on:click={() => select('inputs')}
	isFirst
	hasLine
	selected={$selectedId === 'inputs'}
	label="Inputs"
>
	<div slot="icon">
		<Icon data={faPen} scale={0.8} />
	</div>
</FlowModuleSchemaItem>
