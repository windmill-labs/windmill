<script lang="ts">
	import { RawScript, type FlowModule } from '$lib/gen'
	import { previewResults } from '$lib/stores'
	import { buildExtraLib, objectToTsType, schemaToTsType } from '$lib/utils'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import Editor from './Editor.svelte'
	import EditorBar from './EditorBar.svelte'
	import FlowPreview from './FlowPreview.svelte'
	import FlowBox from './flows/FlowBox.svelte'
	import FlowInputs from './flows/FlowInputs.svelte'
	import FlowModuleHeader from './flows/FlowModuleHeader.svelte'
	import {
		createInlineScriptModule,
		flowStore,
		loadSchema,
		mode,
		pickScript,
		schemasStore
	} from './flows/flowStore'
	import { getPickableProperties, jobsToResults } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'

	export let open: number
	export let i: number
	export let mod: FlowModule
	export let args: Record<string, any> = {}

	let editor: Editor
	let websocketAlive = { pyright: false, black: false, deno: false }
	let pickableProperties: Object | undefined = undefined

	let bigEditor = false

	$: schema = $schemasStore[i]
	$: shouldPick = 'path' in mod.value && mod.value.path === '' && !('language' in mod.value)
	$: pickableProperties = getPickableProperties($flowStore?.schema, args, $previewResults, $mode, i)
	$: extraLib = buildExtraLib(
		schemaToTsType($flowStore?.schema),
		i === 0 ? schemaToTsType($flowStore?.schema) : objectToTsType($previewResults[i])
	)

	// isTrigger should depend on script.is_trigger
</script>

<div id="module-{i}">
	<FlowBox>
		<svelte:fragment slot="header">
			<FlowModuleHeader bind:open {mod} {i} {shouldPick} isTrigger={false} />
		</svelte:fragment>
		<div slot="content">
			{#if open == i}
				{#if shouldPick}
					<FlowInputs
						on:pick={(e) => pickScript(e.detail.path, i)}
						on:new={(e) => createInlineScriptModule(e.detail.language, i, $mode)}
					/>
				{/if}
				{#if mod.value.type === 'rawscript'}
					<div class="mb-2 overflow-hidden">
						<EditorBar {editor} {websocketAlive} lang={mod.value.language ?? 'deno'} />
					</div>
					<div on:mouseleave={() => loadSchema(i)}>
						<Editor
							bind:websocketAlive
							bind:this={editor}
							class="{bigEditor ? 'h-2/3' : 'h-80'} border p-2 rounded"
							bind:code={mod.value.content}
							deno={mod.value.language === RawScript.language.DENO}
							automaticLayout={true}
							on:blur={() => loadSchema(i)}
							formatAction={() => loadSchema(i)}
						/>
						<button
							class="w-full text-center"
							on:click={() => {
								bigEditor = !bigEditor
							}}><Icon data={bigEditor ? faChevronUp : faChevronDown} scale={1.0} /></button
						>
					</div>
					<div class="mt-2 mb-8">
						<p class="text-gray-500 italic">
							Move the focus outside of the text editor to recompute the input schema or press
							Ctrl/Cmd+S
						</p>
					</div>
				{/if}
				{#if !shouldPick}
					<p class="text-lg font-bold text-gray-900 mb-2">Step inputs</p>
					<SchemaForm
						inputTransform={true}
						{schema}
						{extraLib}
						{i}
						bind:pickableProperties
						bind:args={mod.input_transform}
					/>
				{/if}

				{#if !shouldPick}
					<div class="border-b border-gray-200" />
					<div class="p-3">
						<FlowPreview
							bind:args
							flow={$flowStore}
							{i}
							schemas={$schemasStore}
							on:change={(e) => {
								previewResults.set(jobsToResults(e.detail))
							}}
						/>
					</div>
				{/if}
			{/if}
		</div>
	</FlowBox>
</div>
