<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import { faFlagCheckered } from '@fortawesome/free-solid-svg-icons'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<FlowModuleSchemaItem
	on:click={() => ($selectedId = 'inputs')}
	selected={$selectedId === 'inputs'}
	bold
	label="Flow Input"
	id={'flow_input'}
>
	<div slot="icon">
		<Icon data={faFlagCheckered} scale={0.8} />
	</div>
</FlowModuleSchemaItem>
