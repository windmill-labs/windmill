<script lang="ts">
	import type { Schema } from '$lib/common'
	import { FlowModuleValue, type Flow, type FlowModule } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { loadSchema as UloadSchema } from '$lib/scripts'
	import { DENO_INIT_CODE, PYTHON_INIT_CODE } from '$lib/script_helpers'
	import { addPreviewResult, previewResults, workspaceStore } from '$lib/stores'
	import {
		buildExtraLib,
		emptySchema,
		objectToTsType,
		schemaToObject,
		schemaToTsType
	} from '$lib/utils'
	import { language, mehO } from 'svelte-awesome/icons'
	import Editor from './Editor.svelte'
	import FlowPreview from './FlowPreview.svelte'
	import RadioButton from './RadioButton.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import ScriptPicker from './ScriptPicker.svelte'

	export let flow: Flow
	export let i: number
	export let mod: FlowModule
	export let args: Record<string, any> = {}

	export let schemas: Schema[] = []
	export let schemaForms: (SchemaForm | undefined)[] = []

	let editor: Editor
	let flowPreview: FlowPreview

	$: previousSchema = i === 0 ? schemaToObject(flow.schema) : $previewResults[i]

	$: extraLib = buildExtraLib(
		i == 0 ? schemaToTsType(flow.schema) : objectToTsType($previewResults[i])
	)

	function initContent(lang: string) {
		const newStart = lang == 'deno' ? DENO_INIT_CODE : PYTHON_INIT_CODE
		if (editor) {
			editor.setCode(newStart)
		} else {
			mod.value.content = newStart
		}
	}

	$: mod.value.type == 'rawscript' &&
		mod.value.language == undefined &&
		(mod.value.language = FlowModuleValue.language.DENO)

	$: mod.value.type == 'rawscript' && mod.value.language && initContent(mod.value.language)

	export async function loadSchema() {
		let isRaw = mod.value.type == 'rawscript'
		if ((!isRaw && mod.value.path) || isRaw) {
			let schema: Schema
			if (isRaw) {
				schema = emptySchema()
				await inferArgs(mod.value.language!, mod.value.content!, schema)
			} else {
				schema = await UloadSchema(mod.value.path!)
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

	$: $workspaceStore && loadSchema()
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
			<RadioButton
				small={true}
				options={[
					['Pick from an existing script', 'script'],
					['Edit in-place', 'rawscript']
				]}
				bind:value={mod.value.type}
			/>
			{#if mod.value.type == 'script'}
				<ScriptPicker allowHub={true} bind:scriptPath={mod.value.path} on:select={loadSchema} />
			{:else}
				<div class="mt-2" />
				<RadioButton
					label="Language"
					small={true}
					options={[
						['Python 3.10', 'python3'],
						['Typescript (Deno)', 'deno']
					]}
					bind:value={mod.value.language}
				/>
				<div class="h-96 mt-4">
					{#if mod.value.language == 'deno'}
						<Editor
							bind:this={editor}
							class="h-full"
							bind:code={mod.value.content}
							deno={true}
							cmdEnterAction={() => flowPreview.runPreview(mod.input_transform)}
						/>
					{:else}
						<Editor
							bind:this={editor}
							class="h-full"
							bind:code={mod.value.content}
							deno={false}
							cmdEnterAction={() => flowPreview.runPreview(mod.input_transform)}
						/>
					{/if}
				</div>
				<button class="default-button w-full p-1 mt-4" on:click={loadSchema}>Infer schema</button>
			{/if}
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
				bind:this={flowPreview}
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
