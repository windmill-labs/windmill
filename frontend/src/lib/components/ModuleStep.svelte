<script lang="ts">
	import { FlowModuleValue, type FlowModule } from '$lib/gen'
	import { addPreviewResult, previewResults } from '$lib/stores'
	import { buildExtraLib, objectToTsType, schemaToObject, schemaToTsType } from '$lib/utils'
	import Editor from './Editor.svelte'
	import FlowPreview from './FlowPreview.svelte'
	import FlowInputs from './flows/FlowInputs.svelte'
	import FlowModuleHeader from './flows/FlowModuleHeader.svelte'
	import {
		createInlineScriptModule,
		flowStore,
		loadSchema,
		pickScript,
		schemasStore
	} from './flows/flowStore'
	import SchemaForm from './SchemaForm.svelte'

	export let i: number
	export let mod: FlowModule
	export let args: Record<string, any> = {}

	$: schema = $schemasStore[i]
	$: shouldPick = mod.value.path === '' && mod.value.language === undefined
	$: previousSchema = i === 0 ? schemaToObject($flowStore?.schema) : $previewResults[i]
	$: extraLib = buildExtraLib(
		i === 0 ? schemaToTsType($flowStore?.schema) : objectToTsType($previewResults[i])
	)
</script>

<li class="flex flex-row flex-shrink max-w-full mx-auto mt-20">
	<div class="bg-white border border-gray xl-rounded shadow-lg w-full max-w-4xl mx-4 md:mx-auto">
		<div class="flex items-center justify-between flex-wra p-4 sm:px-6">
			<FlowModuleHeader bind:i bind:shouldPick>
				<h3 class="text-lg font-bold text-gray-900">Step {i + 1}</h3>
				<p>{mod.value.path ?? ''}</p>
			</FlowModuleHeader>
		</div>
		<div class="border-b border-gray-200" />
		<div class="p-6">
			{#if shouldPick}
				<FlowInputs
					on:pick={(e) => pickScript(e.detail.path, i)}
					on:new={(e) => createInlineScriptModule(e.detail.language, i)}
				/>
			{/if}
			{#if mod.value.type === FlowModuleValue.type.RAWSCRIPT}
				<div class="h-96">
					<Editor
						class="h-full"
						bind:code={mod.value.content}
						deno={mod.value.language === FlowModuleValue.language.DENO}
					/>
				</div>
				<button class="default-button w-full p-1 mt-4" on:click={() => loadSchema(i)}>
					Infer schema
				</button>
			{/if}
			{#if !shouldPick}
				<h2 class="mb-4">Step inputs</h2>
				<SchemaForm
					inputTransform={true}
					{schema}
					{extraLib}
					{i}
					{previousSchema}
					bind:args={mod.input_transform}
				/>
			{/if}
		</div>

		{#if !shouldPick}
			<div class="border-b border-gray-200" />
			<div class="p-3">
				<FlowPreview
					bind:args
					bind:flow={$flowStore}
					{i}
					bind:schemas={$schemasStore}
					on:change={(e) => {
						addPreviewResult(e.detail.result, i + 1)
					}}
				/>
			</div>
		{/if}
	</div>
</li>
