<script lang="ts">
	import { Button, DrawerContent } from '$lib/components/common'

	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { copyFirstStepSchema } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	import CapturePayload from './CapturePayload.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { convert } from '@redocly/json-to-json-schema'
	import { sendUserToast } from '$lib/toast'
	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import EditableSchemaForm from '$lib/components/EditableSchemaForm.svelte'

	export let noEditor: boolean

	const { flowStore, flowStateStore, previewArgs, initialPath } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let capturePayload: CapturePayload
	let inputLibraryDrawer: Drawer
	let jsonPayload: Drawer
	let pendingJson: string

	function importJson() {
		const parsed = JSON.parse(pendingJson)

		if (!parsed) {
			sendUserToast('Invalid JSON', true)
			return
		}

		$flowStore.schema = { required: [], properties: {}, ...convert(parsed) }

		jsonPayload.closeDrawer()
	}
</script>

<CapturePayload bind:this={capturePayload} />
<FlowCard {noEditor} title="Flow Input">
	<div class="p-6">
		<div class="flex flex-row items-center gap-2 pb-2 border-b border-gray-400">
			<div>Copy input's schema from</div>
			<Button
				color="dark"
				size="xs"
				on:click={() => {
					capturePayload.openDrawer()
				}}
			>
				A request
			</Button>
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
		<div class="pt-6">
			<SchemaEditor
				isFlowInput
				bind:schema={$flowStore.schema}
				on:change={() => {
					$flowStore = $flowStore
				}}
			/>
		</div>
	</div>
	<div class="p-6">
		<h2 class="mb-4">Customize Flow Inputs</h2>
		<EditableSchemaForm bind:schema={$flowStore.schema} />
	</div>
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
