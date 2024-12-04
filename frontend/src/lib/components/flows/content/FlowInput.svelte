<script lang="ts">
	import { Button, DrawerContent } from '$lib/components/common'
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { copyFirstStepSchema } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { convert } from '@redocly/json-to-json-schema'
	import { sendUserToast } from '$lib/toast'
	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import EditableSchemaForm from '$lib/components/EditableSchemaForm.svelte'
	import AddProperty from '$lib/components/schema/AddProperty.svelte'
	import FlowInputViewer from '$lib/components/FlowInputViewer.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import CapturePanel from '$lib/components/triggers/CapturePanel.svelte'
	import { insertNewPreprocessorModule } from '../flowStateUtils'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import InputSchemaPicker from '$lib/components/flows/pickers/InputSchemaPicker.svelte'

	export let noEditor: boolean
	export let disabled: boolean

	const { flowStore, flowStateStore, previewArgs, initialPath, pathStore, selectedId } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let inputLibraryDrawer: Drawer
	let jsonPayload: Drawer
	let pendingJson: string
	let addProperty: AddProperty | undefined = undefined

	function importJson() {
		const parsed = JSON.parse(pendingJson)

		if (!parsed) {
			sendUserToast('Invalid JSON', true)
			return
		}

		$flowStore.schema = { required: [], properties: {}, ...convert(parsed) }

		jsonPayload.closeDrawer()
	}
	const yOffset = 191

	let tabSelected = 'input'
</script>

<FlowCard {noEditor} title="Flow Input">
	{#if !disabled}
		<Tabs bind:selected={tabSelected}>
			<Tab value="input">Input form</Tab>
			<Tab value="capture">Capture</Tab>
		</Tabs>

		{#if tabSelected === 'input'}
			<div class="flex flex-row items-center gap-2 px-4 py-2 border-b">
				<div class="text-sm">Copy input's schema from</div>
				<Popover closeButton={false}>
					<svelte:fragment slot="trigger">
						<Button color="dark" size="xs" nonCaptureEvent>Captures</Button>
					</svelte:fragment>
					<svelte:fragment slot="content">
						<InputSchemaPicker isFlow={true} path={$pathStore} />
					</svelte:fragment>
				</Popover>
				<Button
					color="dark"
					size="xs"
					on:click={() => {
						jsonPayload.openDrawer()
					}}
				>
					A JSON
				</Button>
				<Button
					color="dark"
					size="xs"
					on:click={() => {
						inputLibraryDrawer.openDrawer()
					}}
				>
					Past Runs/Input library
				</Button>
				<Button
					color="dark"
					size="xs"
					disabled={$flowStore.value.modules.length === 0 ||
						$flowStore.value.modules[0].value.type == 'identity'}
					on:click={() => copyFirstStepSchema($flowStateStore, flowStore)}
				>
					First step's inputs
				</Button>
			</div>
			<div class="p-4 border-b">
				<AddProperty
					bind:schema={$flowStore.schema}
					bind:this={addProperty}
					on:change={() => {
						$flowStore = $flowStore
					}}
				/>
			</div>

			<EditableSchemaForm
				bind:schema={$flowStore.schema}
				isFlowInput
				on:edit={(e) => {
					addProperty?.openDrawer(e.detail)
				}}
				on:delete={(e) => {
					addProperty?.handleDeleteArgument([e.detail])
				}}
				offset={yOffset}
				displayWebhookWarning
			/>
		{:else}
			<CapturePanel
				isFlow
				path={$pathStore}
				hasPreprocessor={!!$flowStore.value.preprocessor_module}
				canHavePreprocessor
				newItem={initialPath === ''}
				on:openTriggers
				on:applyArgs
				on:addPreprocessor={async () => {
					await insertNewPreprocessorModule(flowStore, flowStateStore, {
						language: 'bun',
						subkind: 'preprocessor'
					})
					$selectedId = 'preprocessor'
				}}
				on:updateSchema={(e) => {
					const { schema, redirect } = e.detail
					$flowStore.schema = schema
					if (redirect) {
						tabSelected = 'input'
					}
				}}
			/>
		{/if}
	{:else}
		<div class="p-4 border-b">
			<FlowInputViewer schema={$flowStore.schema} />
		</div>
	{/if}
</FlowCard>

<Drawer bind:this={jsonPayload} size="800px">
	<DrawerContent
		title="Input schema from JSON"
		on:close={() => {
			jsonPayload.closeDrawer()
		}}
		noPadding
	>
		<SimpleEditor bind:code={pendingJson} lang="json" class="h-full" />
		<svelte:fragment slot="actions">
			<Button size="sm" on:click={importJson}>Import</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

<Drawer bind:this={inputLibraryDrawer}>
	<DrawerContent title="Input library {initialPath}" on:close={inputLibraryDrawer?.toggleDrawer}>
		<SavedInputs
			flowPath={initialPath}
			isValid={true}
			args={$previewArgs}
			canSaveInputs={false}
			on:selected_args={(e) => {
				const parsed = JSON.parse(JSON.stringify(e.detail))

				if (!parsed) {
					sendUserToast('Invalid JSON', true)
					return
				}

				$flowStore.schema = { required: [], properties: {}, ...convert(parsed) }
				inputLibraryDrawer?.closeDrawer()
			}}
		/>
	</DrawerContent>
</Drawer>
