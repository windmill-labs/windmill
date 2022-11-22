<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar from '$lib/components/EditorBar.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import { createScriptFromInlineScript, fork } from '$lib/components/flows/flowStateUtils'
	import { flowStore } from '$lib/components/flows/flowStore'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { RawScript, type FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { flowStateStore } from '../flowState'
	import { scriptLangToEditorLang } from '$lib/utils'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { afterUpdate, getContext, setContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { loadSchemaFromModule } from '../utils'
	import FlowModuleScript from './FlowModuleScript.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowRetries from './FlowRetries.svelte'
	import { getStepPropPicker } from '../previousResults'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Kbd } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule
	export let failureModule: boolean = false

	export let parentModule: FlowModule | undefined = undefined
	export let previousModule: FlowModule | undefined

	let editor: Editor
	let modulePreview: ModulePreview
	let websocketAlive = { pyright: false, black: false, deno: false, go: false }
	let selected = 'inputs'
	let wrapper: HTMLDivElement
	let panes: HTMLElement
	let totalTopGap = 0
	let validCode = true
	let width = 1200

	let inputTransforms: Record<string, any> =
		flowModule.value.type === 'rawscript' || flowModule.value.type === 'script'
			? flowModule.value.input_transforms
			: {}

	$: if (flowModule.value.type === 'rawscript' || flowModule.value.type === 'script') {
		flowModule.value.input_transforms = inputTransforms
	}

	$: stepPropPicker = failureModule
		? {
				pickableProperties: {
					flow_input: $flowStateStore.previewArgs,
					priorIds: {},
					previousId: undefined
				},
				extraLib: ''
		  }
		: getStepPropPicker(
				$flowStateStore,
				parentModule,
				previousModule,
				flowModule.id,
				$flowStore,
				$previewArgs,
				false,
				true
		  )

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			selected = 'test'
			modulePreview?.runTestWithStepArgs()
		}
	}

	async function reload(flowModule: FlowModule) {
		try {
			const { input_transforms, schema } = await loadSchemaFromModule(flowModule)
			validCode = true
			setTimeout(() => {
				if (
					(flowModule.value.type == 'script' || flowModule.value.type == 'rawscript') &&
					JSON.stringify(flowModule.value.input_transforms) !== JSON.stringify(input_transforms)
				) {
					inputTransforms = input_transforms
				}
			})

			if (JSON.stringify(schema) !== JSON.stringify($flowStateStore[flowModule.id]?.schema)) {
				if (!$flowStateStore[flowModule.id]) {
					$flowStateStore[flowModule.id] = { schema }
				} else {
					$flowStateStore[flowModule.id].schema = schema
				}
			}
		} catch (e) {
			validCode = false
		}
	}

	afterUpdate(() => {
		totalTopGap = 0
		if (!(wrapper && panes)) return

		const wrapperTop = wrapper.getBoundingClientRect().top
		const panesTop = panes.getBoundingClientRect().top
		totalTopGap = panesTop - wrapperTop
	})
</script>

<svelte:window on:keydown={onKeyDown} />

{#if flowModule.value.type === 'rawscript' || flowModule.value.type === 'script'}
	<div class="h-full" bind:this={wrapper} bind:clientWidth={width}>
		<FlowCard bind:flowModule>
			<svelte:fragment slot="header">
				<FlowModuleHeader
					bind:module={flowModule}
					on:toggleSuspend={() => (selected = 'advanced-suspend')}
					on:toggleRetry={() => (selected = 'advanced-retries')}
					on:toggleStopAfterIf={() => (selected = 'advanced-early-stop')}
					on:fork={async () => {
						const [module, state] = await fork(flowModule)
						flowModule = module
						$flowStateStore[module.id] = state
					}}
					on:createScriptFromInlineScript={async () => {
						const [module, state] = await createScriptFromInlineScript(
							flowModule,
							$selectedId,
							$flowStateStore[flowModule.id].schema
						)
						flowModule = module
						$flowStateStore[module.id] = state
					}}
				/>
			</svelte:fragment>

			{#if flowModule.value.type === 'rawscript'}
				<div class="border-b-2 shadow-sm p-1 mb-1">
					<EditorBar
						{validCode}
						{editor}
						lang={flowModule.value['language'] ?? 'deno'}
						{websocketAlive}
						iconOnly={width < 768}
					/>
				</div>
			{/if}

			<div
				bind:this={panes}
				class="h-full"
				style="max-height: calc(100% - {totalTopGap}px) !important;"
			>
				<Splitpanes horizontal>
					<Pane size={50} minSize={20}>
						{#if flowModule.value.type === 'rawscript'}
							<div class="h-full">
								<Editor
									bind:websocketAlive
									bind:this={editor}
									class="h-full px-2"
									bind:code={flowModule.value.content}
									deno={flowModule.value.language === RawScript.language.DENO}
									lang={scriptLangToEditorLang(flowModule.value.language)}
									automaticLayout={true}
									cmdEnterAction={async () => {
										selected = 'test'
										if (flowModule.value.type === 'rawscript') {
											flowModule.value.content = editor.getCode()
										}
										await reload(flowModule)
										modulePreview?.runTestWithStepArgs()
									}}
									on:change={async (event) => {
										await reload(flowModule)
									}}
									formatAction={() => reload(flowModule)}
								/>
							</div>
						{:else if flowModule.value.type === 'script'}
							<FlowModuleScript path={flowModule.value.path} />
						{/if}
					</Pane>
					<Pane size={50} minSize={20}>
						<Tabs bind:selected>
							<Tab value="inputs"><span class="font-semibold">Step Input</span></Tab>
							<Tab value="test"><span class="font-semibold text-md">Test this step</span></Tab>
							<Tab value="advanced">Advanced</Tab>
						</Tabs>
						<div class="h-[calc(100%-32px)]">
							{#if selected === 'inputs'}
								<div class="h-full overflow-auto">
									<PropPickerWrapper pickableProperties={stepPropPicker.pickableProperties}>
										<SchemaForm
											schema={$flowStateStore[$selectedId]?.schema ?? {}}
											inputTransform={true}
											previousModuleId={previousModule?.id}
											bind:args={flowModule.value.input_transforms}
											bind:extraLib={stepPropPicker.extraLib}
										/>
									</PropPickerWrapper>
								</div>
							{:else if selected === 'test'}
								<ModulePreview
									bind:this={modulePreview}
									mod={flowModule}
									schema={$flowStateStore[$selectedId]?.schema ?? {}}
								/>
							{:else if selected.startsWith('advanced')}
								<Tabs bind:selected>
									<Tab value="advanced-retries">Retries</Tab>
									{#if !$selectedId.includes('failure')}
										<Tab value="advanced-early-stop">Early Stop</Tab>
										<Tab value="advanced-suspend">Sleep/Suspend</Tab>
										<Tab value="advanced-same_worker">Same Worker/Shared dir</Tab>
									{/if}
								</Tabs>
								{#if selected === 'advanced-retries'}
									<FlowRetries bind:flowModule class="px-4 pb-4 h-full overflow-auto" />
								{:else if selected === 'advanced-early-stop'}
									<FlowModuleEarlyStop bind:flowModule class="px-4 pb-4 h-full overflow-auto" />
								{:else if selected === 'advanced-suspend'}
									<div class="px-4 pb-4 h-full overflow-auto">
										<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule />
									</div>
								{:else if selected === 'advanced-same_worker'}
									<div class="p-4  h-full overflow-auto">
										<Alert type="info" title="Share a directory using same worker">
											If same worker is set, all steps will be run on the same worker and will share
											the folder `/shared` to pass data between each other.
										</Alert>
										<Button
											btnClasses="mt-4"
											on:click={() => {
												$selectedId = 'settings-same-worker'
											}}>Set same worker in the flow settings</Button
										>
									</div>
								{/if}
							{/if}
						</div>
					</Pane>
				</Splitpanes>
			</div>
		</FlowCard>
	</div>
{:else}
	Incorrect flow module type
{/if}
