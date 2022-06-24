<script lang="ts">
	import type { Schema } from '$lib/common'
	import { loadSchema } from '$lib/scripts'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { FlowModuleValue, type Flow } from '../../gen'
	import FlowPreview from './FlowPreview.svelte'
	import { loadFlowSchemas } from './flows/loadFlowSchemas'
	import ModuleStep from './ModuleStep.svelte'
	import SchemaEditor from './SchemaEditor.svelte'
	import type SchemaForm from './SchemaForm.svelte'

	export let flow: Flow

	let args: Record<string, any> = {}
	let schemas: Schema[] = []
	let schemaForms: (SchemaForm | undefined)[] = []

	function addModule() {
		schemaForms.push(undefined)

		let newModule = {
			value: { type: FlowModuleValue.type.SCRIPT, path: '' },
			input_transform: {}
		}
		flow.value.modules = flow.value.modules.concat(newModule)
		schemas.push(emptySchema())
	}

	async function loadSchemas() {
		schemas = await loadFlowSchemas(flow, $workspaceStore!)
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
								flow.schema = await loadSchema(flow.value.modules[0].value.path ?? '')
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
