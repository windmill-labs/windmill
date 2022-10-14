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
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowRetries from './FlowRetries.svelte'
	import { Button, Tab, TabContent, Tabs } from '$lib/components/common'
	import type { FlowModule } from '$lib/gen/models/FlowModule'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { faTrash, faPlus } from '@fortawesome/free-solid-svg-icons'

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
		<Splitpanes horizontal>
			<Pane size={60} minSize={20} class="px-4 pt-4">
				<div class="h-full overflow-auto">
					{#if mod.value.type === 'forloopflow'}
						<div class="mb-2 text-sm font-bold">
							Iterator expression
							<Tooltip>
								List to iterate over. For more information see the
								<a href="https://docs.windmill.dev/docs/getting_started/flows#for-loops">docs.</a>
							</Tooltip>
						</div>
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
						<div class="mt-6 mb-2 text-sm font-bold">Skip failures</div>
						<Toggle
							bind:checked={mod.value.skip_failures}
							options={{
								right: 'Skip failures'
							}}
						/>
						<div class="mt-6 mb-2 text-sm font-bold">
							Pass specific flow context as loop flow input
						</div>
						<div class="flex flex-row w-80 max-w-full">
							<input
								bind:value={inputTransformName}
								placeholder="Argument name"
								type="text"
								class="w-20"
							/>
							<Button
								size="sm"
								disabled={inputTransformName == ''}
								btnClasses="ml-2"
								startIcon={{ icon: faPlus }}
								iconOnly
								on:click={() => {
									mod.input_transforms[inputTransformName] = { type: 'javascript', expr: '' }
									inputTransformName = ''
								}}
							/>
						</div>
						{#each Object.keys(mod.input_transforms) as key}
							<div class="flex flex-row items-center mt-6 mb-2">
								<span class="my-2 text-sm font-bold">{key}</span>
								<Button
									size="xs"
									variant="border"
									btnClasses="ml-2 !px-2"
									color="red"
									startIcon={{ icon: faTrash }}
									iconOnly
									on:click={() => {
										delete mod.input_transforms[key]
										mod.input_transforms = mod.input_transforms
									}}
								/>
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
				</div>
			</Pane>
			<Pane size={40} minSize={20} class="flex flex-col flex-1">
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
			</Pane>
		</Splitpanes>
	</FlowCard>
</div>
