<script lang="ts">
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { flowStore } from '../flowStore'
	import { getStepPropPicker } from '../flowStateUtils'
	import { flowStateStore } from '../flowState'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { VSplitPane } from 'svelte-split-pane'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowRetries from './FlowRetries.svelte'

	import { Button, Tab, TabContent, Tabs } from '$lib/components/common'
	import type { FlowModule } from '$lib/gen/models/FlowModule'

	const { previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	export let mod: FlowModule
	export let index: number

	let editor: SimpleEditor | undefined = undefined
	let monacos: { [id: string]: SimpleEditor } = {}

	let selected: string = 'retries'

	let inputTransformName = ''

	$: mod = $flowStore.value.modules[index]

	$: pickableProperties = getStepPropPicker(
		[Number(index)],
		$flowStore.schema,
		$flowStateStore,
		$previewArgs
	).pickableProperties
</script>

<div class="h-full flex flex-col">
	<FlowCard title="For loop">
		<div slot="header" class="grow">
			<input bind:value={mod.summary} placeholder={'Summary'} />
		</div>
		<div class="overflow-hidden flex-grow">
			<VSplitPane topPanelSize="60%" downPanelSize="40%" minTopPaneSize="20%" minDownPaneSize="20%">
				<top slot="top" class="h-full">
					<div class="p-6 flex flex-col h-full overflow-clip">
						{#if mod.value.type === 'forloopflow'}
							<span class="mb-2 text-sm font-bold"
								>Iterator expression
								<Tooltip>
									List to iterate over. For more information see the
									<a href="https://docs.windmill.dev/docs/getting_started/flows#for-loops">docs.</a>
								</Tooltip>
							</span>

							{#if mod.value.iterator.type == 'javascript'}
								<div class="border w-full">
									<PropPickerWrapper
										{pickableProperties}
										on:select={({ detail }) => {
											editor?.insertAtCursor(detail)
										}}
									>
										<SimpleEditor
											bind:this={editor}
											lang="javascript"
											bind:code={mod.value.iterator.expr}
											class="small-editor"
											shouldBindKey={false}
										/>
									</PropPickerWrapper>
								</div>
							{:else}
								<Button
									on:click={() => {
										if (mod.value.type === 'forloopflow') mod.value.iterator.type = 'javascript'
									}}
								/>
							{/if}

							<span class="my-2 text-sm font-bold">Skip failures</span>

							<Toggle
								bind:checked={mod.value.skip_failures}
								options={{
									right: 'Skip failures'
								}}
							/>
							<span class="my-2 text-sm font-bold"
								>Pass specific flow context as loop flow input</span
							>
							<div class="flex flex-row mt-4 w-80 max-w-full"
								><input
									bind:value={inputTransformName}
									placeholder="Argument name"
									type="text"
									class="w-20"
								/><Button
									disabled={inputTransformName == ''}
									btnClasses="ml-2"
									on:click={() =>
										(mod.input_transforms[inputTransformName] = { type: 'javascript', expr: '' })}
									>+</Button
								></div
							>

							{#each Object.keys(mod.input_transforms) as key}
								<div class="flex flex-row my-2">
									<span class="my-2 text-sm font-bold">{key}</span>

									<Button
										btnClasses="ml-4"
										on:click={() => {
											delete mod.input_transforms[key]
											mod.input_transforms = mod.input_transforms
										}}>-</Button
									>
								</div>
								<div class="border w-full">
									{#if mod.input_transforms[key].type == 'javascript'}
										<PropPickerWrapper
											{pickableProperties}
											on:select={({ detail }) => {
												monacos[key]?.insertAtCursor(detail)
											}}
										>
											<SimpleEditor
												bind:this={monacos[key]}
												lang="javascript"
												bind:code={mod.input_transforms[key]['expr']}
												class="small-editor"
												shouldBindKey={false}
											/>
										</PropPickerWrapper>
									{:else}
										<Button
											on:click={() => {
												mod.input_transforms[key].type = 'javascript'
												mod.input_transforms[key]['expr'] = ''
											}}
										/>
									{/if}
								</div>
							{/each}
						{/if}
					</div></top
				>
				<down slot="down" class="flex flex-col flex-1 h-full">
					<Tabs bind:selected>
						<Tab value="retries">Retries</Tab>
						<Tab value="early-stop">Early Stop</Tab>
						<Tab value="suspend">Sleep/Suspend</Tab>

						<svelte:fragment slot="content">
							<div class="overflow-hidden bg-white" style="height:calc(100% - 32px);">
								<TabContent value="retries" class="flex flex-col flex-1 h-full">
									<div class="p-4 overflow-y-auto">
										<FlowRetries bind:flowModule={mod} />
									</div>
								</TabContent>

								<TabContent value="early-stop" class="flex flex-col flex-1 h-full">
									<div class="p-4 overflow-y-auto">
										<FlowModuleEarlyStop bind:flowModule={mod} />
									</div>
								</TabContent>

								<TabContent value="suspend" class="flex flex-col flex-1 h-full">
									<div class="p-4 overflow-y-auto">
										<FlowModuleSuspend bind:flowModule={mod} />
									</div>
								</TabContent>
							</div>
						</svelte:fragment>
					</Tabs>
				</down>
			</VSplitPane>
		</div></FlowCard
	>
</div>
