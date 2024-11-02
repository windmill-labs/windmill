<script lang="ts">
	// import EditableSchemaWrapper from '$lib/components/schema/EditableSchemaWrapper.svelte'

	// import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	// import type { OpenFlow } from '$lib/gen'
	// import { writable, type Writable } from 'svelte/store'

	// let schema = {
	// 	type: 'object',
	// 	properties: {
	// 		foo: {
	// 			type: 'string',
	// 			description: '',
	// 			required: [],
	// 			nullable: false,
	// 			default: '',
	// 			disableVariablePicker: false,
	// 			disableCreate: false,
	// 			password: false
	// 		}
	// 	},
	// 	order: ['foo']
	// }

	// let flowStore: Writable<OpenFlow> = writable({
	// 	summary: 'foo',
	// 	value: {
	// 		modules: []
	// 	}
	// })
	// let flowStateStore: Writable<Record<string, any>> = writable({})
</script>

<!-- <EditableSchemaWrapper on:change={() => console.log('FOO')} {schema} /> -->
<!-- <FlowBuilder
	newFlow={false}
	initialPath="u/admin/foo"
	selectedId={undefined}
	{flowStore}
	{flowStateStore}
	customUi={{
		topBar: {
			extraDeployOptions: false
		},
		settingsTabs: {
			workerGroup: false
		},
		settingsPanel: false
	}}
/> -->

<ScriptBuilder
	script={{
		summary: 'foo',
		path: 'u/admin/foo',
		description: 'foo',
		language: 'python3',
		content: 'print("foo")'
	}}
/>
