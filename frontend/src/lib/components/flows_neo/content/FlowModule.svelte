<script lang="ts">
	import type { Schema } from '$lib/common'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar from '$lib/components/EditorBar.svelte'
	import FlowPreview from '$lib/components/FlowPreview.svelte'
	import FlowInputs from '$lib/components/flows/FlowInputs.svelte'
	import FlowModuleHeader from '$lib/components/flows/FlowModuleHeader.svelte'
	import { flowStateStore, type FlowModuleSchema } from '$lib/components/flows/flowState'
	import {
		createInlineScriptModule,
		createLoop,
		createScriptFromInlineScript,
		fork,
		getStepPropPicker,
		isEmptyFlowModule,
		loadFlowModuleSchema,
		pickScript
	} from '$lib/components/flows/flowStateUtils'
	import { flowStore } from '$lib/components/flows/flowStore'
	import SchemaForm from '$lib/components/SchemaForm.svelte'

	import { RawScript, type FlowModule } from '$lib/gen'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import FlowCard from '../common/FlowCard.svelte'

	export let indexes: string
	export let flowModule: FlowModule
	export let args: Record<string, any> = {}
	export let schema: Schema
	export let childFlowModules: FlowModuleSchema[] | undefined = undefined

	const [i] = indexes.split('-').map(Number)

	let editor: Editor
	let websocketAlive = { pyright: false, black: false, deno: false }
	let bigEditor = false

	$: shouldPick = isEmptyFlowModule(flowModule)
	$: stepPropPicker = getStepPropPicker(
		indexes.split('-').map(Number),
		$flowStore.schema,
		$flowStateStore,
		args
	)

	async function apply<T>(fn: (arg: T) => Promise<FlowModuleSchema>, arg: T) {
		const flowModuleSchema = await fn(arg)

		flowModule = flowModuleSchema.flowModule
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
	}
</script>

<FlowCard title="TODO TITLE">
	<svelte:fragment slot="header">
		<FlowModuleHeader
			bind:mod={flowModule}
			indexes={indexes.split('-').map(Number)}
			on:delete
			on:fork={() => apply(fork, flowModule)}
			on:createScriptFromInlineScript={() => {
				apply(createScriptFromInlineScript, {
					flowModule: flowModule,
					suffix: indexes,
					schema
				})
			}}
		/>
	</svelte:fragment>

	<div slot="content">
		{#if shouldPick}
			<FlowInputs
				shouldDisableTriggerScripts={i != 0}
				shouldDisableLoopCreation={indexes.length > 1 || i == 0}
				on:pick={(e) => apply(pickScript, e.detail.path)}
				on:new={(e) =>
					apply(createInlineScriptModule, {
						language: e.detail.language,
						kind: e.detail.kind,
						subkind: e.detail.subkind
					})}
				on:loop={() => applyCreateLoop()}
			/>
		{/if}
		{#if flowModule.value.type === 'rawscript'}
			<div class="mb-2 overflow-hidden">
				<EditorBar {editor} {websocketAlive} lang={flowModule.value.language ?? 'deno'} />
			</div>
			<div on:mouseleave={() => reload(flowModule)}>
				<Editor
					bind:websocketAlive
					bind:this={editor}
					class="{bigEditor ? 'h-2/3' : 'h-80'} border p-2 rounded"
					bind:code={flowModule.value.content}
					deno={flowModule.value.language === RawScript.language.DENO}
					automaticLayout={true}
					formatAction={() => reload(flowModule)}
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
				importPath={indexes}
				bind:pickableProperties={stepPropPicker.pickableProperties}
				bind:args={flowModule.input_transforms}
				bind:extraLib={stepPropPicker.extraLib}
			/>
		{/if}

		{#if !shouldPick}
			<div class="border-b border-gray-200" />
			<div class="pt-2">
				<FlowPreview bind:args flow={$flowStore} {i} {schema} />
			</div>
		{/if}
	</div>
</FlowCard>
