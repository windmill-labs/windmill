<script lang="ts">
	import { FlowModuleValue, type FlowModule } from '$lib/gen'
	import { previewResults } from '$lib/stores'
	import { buildExtraLib, objectToTsType, schemaToTsType } from '$lib/utils'
	import { faRobot } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import Editor from './Editor.svelte'
	import EditorBar from './EditorBar.svelte'
	import FlowPreview from './FlowPreview.svelte'
	import FlowInputs from './flows/FlowInputs.svelte'
	import FlowModuleHeader from './flows/FlowModuleHeader.svelte'
	import {
		createInlineScriptModule,
		flowStore,
		loadSchema,
		pickScript,
		schemasStore,
		type FlowMode
	} from './flows/flowStore'
	import { getPickableProperties } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import Tooltip from './Tooltip.svelte'

	export let open: number
	export let mode: FlowMode
	export let i: number
	export let mod: FlowModule
	export let args: Record<string, any> = {}

	let editor: Editor
	let websocketAlive = { pyright: false, black: false, deno: false }
	let pickableProperties: Object | undefined = undefined

	$: schema = $schemasStore[i]
	$: shouldPick = mod.value.path === '' && mod.value.language === undefined
	$: pickableProperties = getPickableProperties($flowStore?.schema, args, $previewResults, mode, i)
	$: extraLib = buildExtraLib(
		i === 0 ? schemaToTsType($flowStore?.schema) : objectToTsType($previewResults[i])
	)
</script>

<li class="flex flex-row flex-shrink max-w-full mx-auto mt-16">
	<div
		class="bg-white border border-gray xl-rounded shadow-lg w-full max-w-4xl mx-4 md:mx-auto"
		id="module-{i}"
	>
		<div class="flex items-center justify-between flex-wra p-4 sm:px-6 z-10">
			<FlowModuleHeader bind:open {mod} {i} {shouldPick}>
				<div>
					<h3 class="text-lg font-bold text-gray-900">Step {i + 1}</h3>
					{#if i == 0 && mode == 'pull'}
						<h3 class="font-bold">
							Trigger Script <Tooltip
								>When a flow is 'Pull', the first step is a trigger script. Trigger scripts are
								scripts that must return a list which are the new items to be treated one by one by
								the rest of the flow, usually the list of new items since last time the flow was
								run. One can retrieve the item in the next step using `previous_result._value`. To
								easily compute the diff, windmill provides some helpers under the form of
								`getInternalState` and `setInternalState`.</Tooltip
							>
						</h3>{/if}
					<p>
						{#if mod.value.path}
							{mod.value.path}
						{/if}
						{#if mod.value.language}
							Inline {mod.value.language}
						{/if}
						{#if !mod.value.path && !mod.value.language}
							Select a script
						{/if}
					</p>
				</div>
			</FlowModuleHeader>
		</div>
		<div class="border-b border-gray-200" />
		{#if open == i}
			<div class="p-6">
				{#if shouldPick}
					<FlowInputs
						isTrigger={mode == 'pull' && i == 0}
						on:pick={(e) => pickScript(e.detail.path, i)}
						on:new={(e) => createInlineScriptModule(e.detail.language, i, mode)}
					/>
				{/if}
				{#if mod.value.type === FlowModuleValue.type.RAWSCRIPT}
					<div class="p-1 overflow-hidden">
						<EditorBar {editor} {websocketAlive} lang={mod.value.language ?? 'deno'} />
					</div>
					<Editor
						bind:websocketAlive
						bind:this={editor}
						class="h-80 border p-2 rounded"
						bind:code={mod.value.content}
						deno={mod.value.language === FlowModuleValue.language.DENO}
					/>
					<div class="mt-2 mb-8">
						<button class="default-primary-button-v2" on:click={() => loadSchema(i)}>
							<Icon data={faRobot} class="w-4 h-4 mr-2 -ml-2" />

							Infer step inputs from code
						</button>
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
			</div>

			{#if !shouldPick}
				<div class="border-b border-gray-200" />
				<div class="p-3">
					<FlowPreview
						bind:args
						flow={$flowStore}
						{i}
						{mode}
						schemas={$schemasStore}
						on:change={(e) => {
							const results = e.detail.map((x) => x.result)
							previewResults.set(results)
						}}
					/>
				</div>
			{/if}
		{/if}
		{#if open == i}
			<div>
				<button class="w-full h-full" on:click={() => (open = -1)}>(-)</button>
			</div>
		{:else}
			<div>
				<button class="w-full h-full" on:click={() => (open = i)}>(+)</button>
			</div>
		{/if}
	</div>
</li>
{#if i == 0 && mode == 'pull'}
	<li class="relative m-20 ">
		<div class="relative flex justify-center bg-white shadow p-2">
			Starting from here, the flow for loop over items from step 1's result above &nbsp;<Tooltip
				>This flow being in 'Pull' mode, the rest of the flow will for loop over the list of items
				returned by the trigger script right above. Retrieve the item value using
				`flow_input._value`</Tooltip
			>
		</div>
	</li>
{/if}
