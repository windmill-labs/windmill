<script lang="ts">
	import { FlowModuleValue, type FlowModule } from '$lib/gen'
	import { previewResults } from '$lib/stores'
	import { buildExtraLib, objectToTsType, schemaToTsType } from '$lib/utils'
	import { faChevronDown, faChevronUp, faPlus, faRobot } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import Editor from './Editor.svelte'
	import EditorBar from './EditorBar.svelte'
	import FlowPreview from './FlowPreview.svelte'
	import FlowBox from './flows/FlowBox.svelte'
	import FlowInputs from './flows/FlowInputs.svelte'
	import FlowModuleHeader from './flows/FlowModuleHeader.svelte'
	import {
		addModule,
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
		schemaToTsType($flowStore?.schema),
		i === 0 ? schemaToTsType($flowStore?.schema) : objectToTsType($previewResults[i])
	)

	function toResults(jobs: any) {
		return jobs.map((job) => {
			if (job.result) {
				return job.result
			} else if (Array.isArray(job)) {
				return toResults(job)
			}
		})
	}

	const isTrigger = mode === 'pull' && i === 0
</script>

<button
	class="rounded-full h-10 w-10 border bg-white border-2 border-gray-500"
	on:click={() => {
		addModule(i)
		open = i
	}}
>
	<Icon class="text-gray-500 mb-1" data={faPlus} />
</button>
<FlowBox>
	<div id="module-{i}">
		<FlowModuleHeader bind:open {mod} {i} {shouldPick}>
			<div>
				<h3 class="text-lg font-bold text-gray-900">Step {i + 1}</h3>
				{#if isTrigger}
					<h3 class="font-bold">
						Trigger Script
						<Tooltip>
							When a flow is 'Pull', the first step is a trigger script. Trigger scripts are scripts
							that must return a list which are the new items to be treated one by one by the rest
							of the flow, usually the list of new items since last time the flow was run. One can
							retrieve the item in the next step using `previous_result._value`. To easily compute
							the diff, windmill provides some helpers under the form of `getInternalState` and
							`setInternalState`.
						</Tooltip>
					</h3>
				{/if}
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
		{#if open == i}
			<div class="p-6">
				{#if shouldPick}
					<FlowInputs
						{isTrigger}
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
							previewResults.set(toResults(e.detail))
						}}
					/>
				</div>
			{/if}
		{/if}
		{#if open == i}
			<div>
				<button class="w-full h-full" on:click={() => (open = -1)}>
					<Icon data={faChevronUp} scale={1.0} />
				</button>
			</div>
		{:else}
			<div>
				<button class="w-full h-full" on:click={() => (open = i)}>
					<Icon data={faChevronDown} scale={1.0} />
				</button>
			</div>
		{/if}
	</div>
</FlowBox>
