<script lang="ts">
	import { RawScript, type FlowModule } from '$lib/gen'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import Editor from './Editor.svelte'
	import EditorBar from './EditorBar.svelte'
	import FlowPreview from './FlowPreview.svelte'
	import FlowBox from './flows/FlowBox.svelte'
	import FlowInputs from './flows/FlowInputs.svelte'
	import FlowModuleHeader from './flows/FlowModuleHeader.svelte'
	import { flowStore } from './flows/flowStore'
	import {
		createInlineScriptModule,
		createLoop,
		fork,
		loadFlowModuleSchema,
		pickScript,
		createScriptFromInlineScript,
		isEmptyFlowModule,
		getStepPropPicker
	} from './flows/flowStateUtils'
	import SchemaForm from './SchemaForm.svelte'
	import type { Schema } from '$lib/common'
	import { flowStateStore, type FlowModuleSchema } from './flows/flowState'
	import { stepOpened } from './flows/stepOpenedStore'

	export let indexes: number[]
	export let mod: FlowModule
	export let args: Record<string, any> = {}
	export let schema: Schema
	export let childFlowModules: FlowModuleSchema[] | undefined = undefined

	let editor: Editor
	let websocketAlive = { pyright: false, black: false, deno: false }
	let bigEditor = false

	const i = indexes[0]

	$: shouldPick = isEmptyFlowModule(mod)
	$: stepPropPicker = getStepPropPicker(indexes, $flowStore.schema, $flowStateStore, args)

	async function apply<T>(fn: (arg: T) => Promise<FlowModuleSchema>, arg: T) {
		const flowModuleSchema = await fn(arg)

		mod = flowModuleSchema.flowModule
		schema = flowModuleSchema.schema

		if (flowModuleSchema.childFlowModules) {
			childFlowModules = flowModuleSchema.childFlowModules
		}
	}

	async function reload(flowModule: FlowModule) {
		apply(loadFlowModuleSchema, flowModule)
	}

	async function applyCreateLoop() {
		await apply(createLoop, null)
		stepOpened.update(() => `${indexes[0]}-0`)
	}

	$: opened = $stepOpened === String(indexes.join('-'))
</script>

<FlowBox
	headerClickable={true}
	on:clickheader={() => ($stepOpened = !opened ? String(indexes.join('-')) : undefined)}
>
	<svelte:fragment slot="header">
		<FlowModuleHeader
			bind:mod
			bind:indexes
			on:delete
			on:fork={() => apply(fork, mod)}
			on:createScriptFromInlineScript={() => {
				apply(createScriptFromInlineScript, {
					flowModule: mod,
					suffix: indexes.join('-'),
					schema
				})
			}}
		/>
	</svelte:fragment>

	<div slot="content">
		{#if opened}
			<div class="p-6 border-t border-gray-300">
				{#if shouldPick}
					<FlowInputs
						shouldDisableTriggerScripts={i != 0}
						shouldDisableLoopCreation={indexes.length > 1 || i == 0}
						on:pick={(e) => apply(pickScript, e.detail.path)}
						on:new={(e) =>
							apply(createInlineScriptModule, {
								language: e.detail.language,
								type: e.detail.type
							})}
						on:loop={() => applyCreateLoop()}
					/>
				{/if}
				{#if mod.value.type === 'rawscript'}
					<div class="mb-2 overflow-hidden">
						<EditorBar {editor} {websocketAlive} lang={mod.value.language ?? 'deno'} />
					</div>
					<div on:mouseleave={() => reload(mod)}>
						<Editor
							bind:websocketAlive
							bind:this={editor}
							class="{bigEditor ? 'h-2/3' : 'h-80'} border p-2 rounded"
							bind:code={mod.value.content}
							deno={mod.value.language === RawScript.language.DENO}
							automaticLayout={true}
							formatAction={() => reload(mod)}
						/>
						<button
							class="w-full text-center"
							on:click={() => {
								bigEditor = !bigEditor
							}}
						>
							<Icon data={bigEditor ? faChevronUp : faChevronDown} scale={1.0} />
						</button>
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
						{schema}
						inputTransform={true}
						importPath={String(indexes.join('-'))}
						bind:pickableProperties={stepPropPicker.pickableProperties}
						bind:args={mod.input_transform}
						bind:extraLib={stepPropPicker.extraLib}
					/>
				{/if}

				{#if !shouldPick}
					<div class="border-b border-gray-200" />
					<div class="p-3">
						<FlowPreview bind:args flow={$flowStore} {i} {schema} />
					</div>
				{/if}
			</div>
		{/if}
	</div>
</FlowBox>
