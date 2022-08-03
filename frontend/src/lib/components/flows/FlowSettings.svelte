<script lang="ts">
	import { sendUserToast } from '$lib/utils'
	import { faFileExport, faFileImport, faGlobe } from '@fortawesome/free-solid-svg-icons'
	import { Dropdown, DropdownItem } from 'flowbite-svelte'
	import Icon from 'svelte-awesome'
	import Editor from '../Editor.svelte'
	import FlowPreview from '../FlowPreview.svelte'
	import FlowViewer from '../FlowViewer.svelte'
	import Modal from '../Modal.svelte'
	import RadioButton from '../RadioButton.svelte'
	import CollapseLink from './../CollapseLink.svelte'
	import CronInput from './../CronInput.svelte'
	import FlowBox from './../flows/FlowBox.svelte'
	import { flowStore, initFlow, mode } from './../flows/flowStore'
	import { flowToMode } from './../flows/utils'
	import Path from './../Path.svelte'
	import Required from './../Required.svelte'
	import SchemaForm from './../SchemaForm.svelte'
	import Toggle from './../Toggle.svelte'
	import Tooltip from './../Tooltip.svelte'
	import FlowBoxHeader from './FlowBoxHeader.svelte'

	export let pathError = ''
	export let initialPath: string = ''

	export let scheduleArgs: Record<string, any> = {}
	export let scheduleEnabled = false
	export let scheduleCron: string = '0 */5 * * *'
	export let open: number

	let jsonSetter: Modal
	let jsonViewer: Modal

	let jsonValue: string = ''
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
				Object.assign($flowStore, JSON.parse(jsonValue))
				initFlow($flowStore)
				open = -1
				sendUserToast('OpenFlow imported from JSON')
				jsonSetter.closeModal()
			}}
		>
			Import
		</button>
	</div>
</Modal>

<Modal bind:this={jsonViewer}>
	<div slot="title">See JSON</div>
	<div slot="content" class="h-full">
		<FlowViewer flow={flowToMode($flowStore, $mode)} tab="json" />
	</div>
</Modal>

<FlowBox>
	<FlowBoxHeader title="Flow Settings">
		<div class="flex flex-row-reverse">
			<Dropdown class="w-fit" placement="bottom-end">
				<button slot="trigger" class="text-gray-900 bg-white dark:text-white dark:bg-gray-800">
					...
				</button>
				<DropdownItem
					on:click={() => {
						jsonSetter.openModal()
					}}
				>
					<Icon data={faFileImport} scale={0.6} class="inline mr-2" />
					Import from a JSON OpenFlow
				</DropdownItem>
				<DropdownItem
					on:click={() => {
						jsonViewer.openModal()
					}}
				>
					<Icon data={faFileExport} scale={0.6} class="inline mr-2" />
					Export to a JSON OpenFlow
				</DropdownItem>
				<DropdownItem
					on:click={() => {
						const url = new URL('https://hub.windmill.dev/flows/add')
						const openFlow = {
							value: $flowStore.value,
							summary: $flowStore.summary,
							description: $flowStore.description,
							schema: $flowStore.schema
						}
						url.searchParams.append('flow', btoa(JSON.stringify(flowToMode(openFlow, $mode))))
						window.open(url, '_blank')?.focus()
					}}
				>
					<Icon data={faGlobe} scale={0.6} class="inline mr-2" />
					Publish to Hub
				</DropdownItem>
			</Dropdown>
		</div>
	</FlowBoxHeader>

	<div class="p-6">
		<Path
			bind:error={pathError}
			bind:path={$flowStore.path}
			{initialPath}
			namePlaceholder="my_flow"
			kind="flow"
		>
			<div slot="ownerToolkit">
				Flow permissions depend on their path. Select the group <span class="font-mono">all</span>
				to share your flow, and <span class="font-mono">user</span> to keep it private.
				<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
			</div>
		</Path>

		<label class="block mt-4">
			<span class="text-gray-700">Summary <Required required={false} /></span>
			<textarea
				bind:value={$flowStore.summary}
				class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
				placeholder="A very short summary of the flow displayed when the flow is listed"
				rows="1"
			/>
		</label>
	</div>

	<div class="mx-6">
		<RadioButton
			options={[
				[
					{
						title: 'UI or webhook triggered',
						desc: 'Trigger this flow through the generated UI, a manual schedule or by calling the associated webhook'
					},
					'push'
				],
				[
					{
						title: 'Watching changes regularly',
						desc: 'The first module of this flow is a trigger script whose purpose is to pull data from an external source and return all new items since last run. This flow is meant to be scheduled very regularly to reduce latency to react to new events. It will trigger the rest of the flow once per item. If no new items, the flow will be skipped.'
					},
					'pull'
				]
			]}
			bind:value={$mode}
		/>
	</div>
	{#if $mode == 'pull'}
		<div class="p-4">
			<CollapseLink text="set primary schedule" open={true}>
				<Tooltip>
					The primary schedule of a flow is simply a schedule that has the same name as a flow. It
					can be set and enabled directly within the flow editor. "Watching for new changes" flows
					are meant to be watching regularly for new items in an external systems. The primary
					schedule purpose is there to set the periodicity at which you want this watcher to
					operate.
				</Tooltip>
				<Toggle
					bind:checked={scheduleEnabled}
					options={{
						left: 'disabled',
						right: 'enabled'
					}}
				/>
				<div class="p-2 mt-2 rounded" class:bg-gray-300={!scheduleEnabled}>
					{#if !scheduleEnabled}
						<span class="font-black">No next scheduled run when disabled</span>
					{/if}
					<CronInput bind:schedule={scheduleCron} />
				</div>
				<SchemaForm schema={$flowStore.schema} bind:args={scheduleArgs} />
			</CollapseLink>
		</div>
	{/if}

	<div class="p-6">
		<FlowPreview
			mode={$mode}
			flow={$flowStore}
			i={$flowStore?.value.modules.length}
			bind:args={scheduleArgs}
		/>
	</div>
</FlowBox>
