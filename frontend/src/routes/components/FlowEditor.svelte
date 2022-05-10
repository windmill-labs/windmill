<script lang="ts">
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import type { Schema } from '../../common'
	import { emptySchema } from '../../utils'

	import Icon from 'svelte-awesome'

	import { type Flow, FlowModuleValue, ScriptService } from '../../gen'
	import SchemaEditor from './SchemaEditor.svelte'
	import type SchemaForm from './SchemaForm.svelte'
	import { workspaceStore } from '../../stores'
	import ModuleStep from './ModuleStep.svelte'
	import FlowPreview from './FlowPreview.svelte'

	export let flow: Flow

	let args: Record<string, any> = {}
	let schemas: Schema[] = []
	let schemaForms: (SchemaForm | undefined)[] = []

	export async function loadSchemas() {
		await Promise.all(
			flow.value.modules.map(async (x, i) => {
				if (x.value.path) {
					const script = await ScriptService.getScriptByPath({
						workspace: $workspaceStore!,
						path: x.value.path ?? ''
					})
					if (
						JSON.stringify(Object.keys(script.schema?.properties ?? {}).sort()) !=
						JSON.stringify(Object.keys(x.input_transform).sort())
					) {
						let it = {}
						Object.keys(script.schema?.properties ?? {}).map(
							(x) =>
								(it[x] = {
									type: 'static',
									value: ''
								})
						)
						schemaForms[i]?.setArgs(it)
					}
					schemas[i] = script.schema ?? emptySchema()
				} else {
					schemaForms[i]?.setArgs({})
					schemas[i] = emptySchema()
				}
			})
		)
		schemas = schemas

		if (flow.value.modules.length == 0) {
			addModule()
		}
	}

	function addModule() {
		schemaForms.push(undefined)

		let newModule = {
			value: { type: FlowModuleValue.type.SCRIPT, path: '' },
			input_transform: {}
		}
		flow.value.modules = flow.value.modules.concat(newModule)
		schemas.push(emptySchema())
	}

	$: $workspaceStore && loadSchemas()
</script>

<!-- <PageHeader title="Flow" /> -->
<div class="flow-root bg-gray-50 rounded-xl border  border-gray-200">
	<ul class="relative -mt-10">
		<span class="absolute top-0 left-1/2  h-full w-1 bg-gray-400" aria-hidden="true" />
		<div class="relative">
			<li class="flex flex-row flex-shrink max-w-full  mx-auto mt-20">
				<div class="bg-white border border-gray xl-rounded shadow-lg w-full mx-4 xl:mx-20">
					<div
						class="flex items-center justify-between flex-wra px-4 py-5 border-b border-gray-200 sm:px-6"
					>
						<h3 class="text-lg leading-6 font-medium text-gray-900">Flow Input</h3>
						<button
							class="text-xs default-button-secondary max-h-6 place-self-end"
							disabled={flow.value.modules.length == 0 ||
								flow.value.modules[0].value.path == undefined}
							on:click={async () => {
								const script = await ScriptService.getScriptByPath({
									workspace: $workspaceStore ?? '',
									path: flow.value.modules[0].value.path ?? ''
								})
								flow.schema = script.schema
							}}
							>Copy from step 1's schema
						</button>
					</div>
					<div class="p-4">
						<SchemaEditor bind:schema={flow.schema} />
						<div class="my-4" />
						<FlowPreview {flow} i={flow.value.modules.length - 1} bind:args />
					</div>
				</div>
			</li>
			{#each flow.value.modules as mod, i}
				<ModuleStep bind:flow bind:mod bind:schemas bind:schemaForms bind:args {i} />
			{/each}
			<li class="relative m-20 ">
				<div class="absolute inset-0 flex items-center" aria-hidden="true">
					<div class="w-full border-t border-gray-300" />
				</div>
				<div class="relative flex justify-center">
					<button
						class="default-button h-10 w-10 shadow-blue-600/40  border-blue-600 shadow"
						on:click={addModule}><Icon class="text-white mb-1" data={faPlus} scale={0.9} /></button
					>
				</div>
			</li>
		</div>
	</ul>
</div>
<div class="py-10 bg-white" />
