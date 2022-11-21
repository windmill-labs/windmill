<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import { faFlagCheckered, faPen } from '@fortawesome/free-solid-svg-icons'

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<FlowModuleSchemaItem
	on:click={() => select('inputs')}
	isFirst
	hasLine
	selected={$selectedId === 'inputs'}
	bold
	label="Flow Input"
	id={'flow_input'}
>
	<div slot="icon">
		<Icon data={faFlagCheckered} scale={0.8} />
	</div>
</FlowModuleSchemaItem>
