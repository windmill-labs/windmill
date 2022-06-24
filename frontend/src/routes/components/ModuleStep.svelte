<script lang="ts">
	import type { Schema } from '../../common'
	import { ScriptService, type Flow, type FlowModule } from '../../gen'
	import { inferArgs } from '../../infer'
	import { addPreviewResult, previewResults, workspaceStore } from '../../stores'
	import {
		buildExtraLib,
		emptySchema,
		objectToTsType,
		schemaToObject,
		schemaToTsType
	} from '../../utils'
	import FlowPreview from './FlowPreview.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import ScriptPicker from './ScriptPicker.svelte'

	export let flow: Flow
	export let i: number
	export let mod: FlowModule
	export let args: Record<string, any> = {}

	export let schemas: Schema[] = []
	export let schemaForms: (SchemaForm | undefined)[] = []

	$: previousSchema = i === 0 ? schemaToObject(flow.schema) : $previewResults[i]

	$: extraLib = buildExtraLib(
		i == 0 ? schemaToTsType(flow.schema) : objectToTsType($previewResults[i])
	)

	export async function loadSchema() {
		if (mod.value.path) {
			let schema
			if (mod.value.path.startsWith('hub/')) {
				const code = await ScriptService.getHubScriptContentByPath({ path: mod.value.path })
				schema = emptySchema()
				await inferArgs('deno', code, schema)
			} else {
				const script = await ScriptService.getScriptByPath({
					workspace: $workspaceStore!,
					path: mod.value.path ?? ''
				})
				schema = script.schema
			}
			if (
				JSON.stringify(Object.keys(schema?.properties ?? {}).sort()) !=
				JSON.stringify(Object.keys(mod.input_transform).sort())
			) {
				let it = {}
				Object.keys(schema?.properties ?? {}).map(
					(x) =>
						(it[x] = {
							type: 'static',
							value: ''
						})
				)
				schemaForms[i]?.setArgs(it)
			}
			schemas[i] = schema ?? emptySchema()
		} else {
			schemaForms[i]?.setArgs({})
			schemas[i] = emptySchema()
		}

		schemas = schemas
	}
</script>

<li class="flex flex-row flex-shrink max-w-full  mx-auto mt-20">
	<div class="bg-white border border-gray xl-rounded shadow-lg w-full max-w-4xl mx-4 md:mx-auto">
		<div
			class="flex items-center justify-between flex-wra px-4 py-5 border-b border-gray-200 sm:px-6"
		>
			<h3 class="text-lg leading-6 font-medium text-gray-900">Step {i + 1}</h3>
			<button
				class="text-xs default-button-secondary max-h-6 place-self-end"
				on:click={() => {
					flow.value.modules.splice(i, 1)
					schemas.splice(i, 1)
					schemaForms.splice(i, 1)
					flow = flow
				}}
				>Remove this step
			</button>
		</div>
		<div class="p-10">
			<h2 class="mb-4">Step script</h2>
			<ScriptPicker allowHub={true} bind:scriptPath={mod.value.path} on:select={loadSchema} />
			<div class="my-4" />
			<h2 class="mb-4">Step inputs</h2>
			<SchemaForm
				bind:this={schemaForms[i]}
				inputTransform={true}
				schema={schemas[i]}
				{extraLib}
				{i}
				{previousSchema}
				bind:args={mod.input_transform}
			/>
			<FlowPreview
				{flow}
				{i}
				bind:args
				{schemas}
				on:change={(e) => {
					addPreviewResult(e.detail.result, i + 1)
				}}
			/>
		</div>
	</div>
</li>
