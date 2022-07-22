<script lang="ts">
	import { FlowModuleValue } from '$lib/gen'

	import { faFileExport, faFileImport, faPlus } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import FlowPreview from './FlowPreview.svelte'
	import CopyFirstStepSchema from './flows/CopyFirstStepSchema.svelte'
	import { addModule, flowStore, type FlowMode } from './flows/flowStore'
	import ModuleStep from './ModuleStep.svelte'
	import Path from './Path.svelte'
	import RadioButtonV2 from './RadioButtonV2.svelte'
	import SchemaEditor from './SchemaEditor.svelte'
	import Required from './Required.svelte'
	import { pathIsEmpty, sendUserToast } from '$lib/utils'
	import Dropdown from './Dropdown.svelte'
	import Editor from './Editor.svelte'
	import Modal from './Modal.svelte'
	import FlowViewer from './FlowViewer.svelte'

	export let pathError = ''
	export let initialPath: string = ''

	let jsonSetter: Modal
	let jsonViewer: Modal

	let jsonValue: string = ''

	let open = 0
	let args: Record<string, any> = {}
	export let mode: FlowMode =
		$flowStore?.value.modules[1]?.value.type == FlowModuleValue.type.FORLOOPFLOW ? 'pull' : 'push'
	$: numberOfSteps = $flowStore?.value.modules.length - 1
</script>

<Modal bind:this={jsonSetter}>
	<div slot="title">Import JSON</div>
	<div slot="content" class="h-full">
		<Editor bind:code={jsonValue} lang={'json'} class="h-full" />
	</div>
	<div slot="submission">
		<button
			class="default-button px-4 py-2 font-semibold"
			on:click={() => {
				$flowStore.value = JSON.parse(jsonValue)
				sendUserToast('Flow imported from JSON')
				jsonSetter.closeModal()
			}}
		>
			Import
		</button>
	</div></Modal
>

<Modal bind:this={jsonViewer}>
	<div slot="title">See JSON</div>
	<div slot="content" class="h-full">
		<FlowViewer flow={$flowStore} tab="json" />
	</div>
</Modal>

<div class="flow-root bg-gray-50 rounded-xl border  border-gray-200">
	<ul class="relative -mt-10">
		<span class="absolute top-0 left-1/2  h-full w-1 bg-gray-400" aria-hidden="true" />
		<div class="relative">
			<li class="flex flex-row flex-shrink max-w-full mx-auto mt-20">
				<div
					class="bg-white border border-gray xl-rounded shadow-lg w-full max-w-4xl mx-4 md:mx-auto p-4"
				>
					<div class="flex flex-row-reverse mr-4">
						<Dropdown
							dropdownItems={[
								{
									displayName: 'Import from JSON',
									icon: faFileImport,
									action: () => {
										jsonSetter.openModal()
									}
								},
								{
									displayName: 'Export to JSON',
									icon: faFileExport,
									action: () => {
										jsonViewer.openModal()
									}
								}
							]}
							relative={false}
						/>
					</div>
					<div class="mb-8 p-4">
						<Path
							bind:error={pathError}
							bind:path={$flowStore.path}
							{initialPath}
							namePlaceholder="example/my/flow"
							kind="flow"
						>
							<div slot="ownerToolkit">
								Flow permissions depend on their path. Select the group <span class="font-mono"
									>all</span
								>
								to share your flow, and <span class="font-mono">user</span> to keep it private.
								<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
							</div>
						</Path>

						<label class="block mt-4">
							<span class="text-gray-700">Summary <Required required={false} /></span>
							<textarea
								bind:value={$flowStore.summary}
								class="
					mt-1
					block
					w-full
					rounded-md
					border-gray-300
					shadow-sm
					focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
					"
								placeholder="A very short summary of the flow displayed when the flow is listed"
								rows="1"
							/>
						</label>
					</div>
					<RadioButtonV2
						options={[
							[
								{
									title: 'Push',
									desc: 'Trigger this flow through the generated UI, a manual schedule or by calling the associated webhook'
								},
								'push'
							],
							[
								{
									title: 'Pull',
									desc: 'The first module of this flow is a trigger script whose purpose is to pull data from an external source and return all new items since last run. This flow is meant to be scheduled very regularly to reduce latency to react to new events. It will trigger the rest of the flow once per item. If no new items, the flow will be skipped.'
								},
								'pull'
							]
						]}
						bind:value={mode}
					/>
				</div>
			</li>
			<li class="flex flex-row flex-shrink max-w-full mx-auto mt-20">
				<div class="bg-white border border-gray xl-rounded shadow-lg w-full mx-4 xl:mx-20">
					<div
						class="flex items-center justify-between flex-wra px-4 py-5 border-b border-gray-200 sm:px-6"
					>
						<h3 class="text-lg leading-6 font-medium text-gray-900">Flow Input</h3>
						<CopyFirstStepSchema />
					</div>
					<div class="p-4">
						<SchemaEditor schema={$flowStore.schema} />
						<div class="my-4" />
						<FlowPreview {mode} flow={$flowStore} i={numberOfSteps} bind:args />
					</div>
				</div>
			</li>
			{#each $flowStore?.value.modules as mod, i}
				<li class="relative mt-16">
					<div class="relative flex justify-center">
						<button
							class="default-button h-10 w-10 shadow-blue-600/40  border-blue-600 shadow"
							on:click={() => {
								addModule(i)
								open = i
							}}
						>
							<Icon class="text-white mb-1" data={faPlus} />
						</button>
					</div>
				</li>
				<ModuleStep bind:open bind:mod bind:args {i} {mode} />
			{/each}
			<li class="relative m-20 ">
				<div class="relative flex justify-center">
					<button
						disabled={pathIsEmpty($flowStore.path)}
						class="default-button h-10 w-10 shadow"
						on:click={() => {
							addModule()
							open = $flowStore?.value.modules.length - 1
						}}
					>
						<Icon class="text-white mb-1" data={faPlus} />
						Add step {pathIsEmpty($flowStore.path) ? '(pick a name first!)' : ''}
					</button>
				</div>
			</li>
		</div>
	</ul>
</div>
<div class="py-10 bg-white" />

<style>
	.shadow:not([disabled]) {
		@apply border-blue-600 shadow-blue-600/40;
	}
</style>
