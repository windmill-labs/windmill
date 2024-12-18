<script lang="ts">
	import { Button } from '$lib/components/common'
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { copyFirstStepSchema } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { convert } from '@redocly/json-to-json-schema'
	import { sendUserToast } from '$lib/toast'
	import EditableSchemaForm from '$lib/components/EditableSchemaForm.svelte'
	import AddPropertyV2 from '$lib/components/schema/AddPropertyV2.svelte'
	import FlowInputViewer from '$lib/components/FlowInputViewer.svelte'
	import HistoricInpts from '$lib/components/HistoricInpts.svelte'
	import SavedInputsPickerV2 from '$lib/components/SavedInputsPickerV2.svelte'
	import { CornerDownLeft, Pen, X, ChevronDown, Plus } from 'lucide-svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import FlowInputEditor from './FlowInputEditor.svelte'
	import CapturesInputs from '$lib/components/CapturesInputs.svelte'
	import CaptureButton from '$lib/components/triggers/CaptureButton.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'
	import ButtonDropDown from '$lib/components/meltComponents/ButtonDropDown.svelte'
	import { tick } from 'svelte'

	export let noEditor: boolean
	export let disabled: boolean

	const { flowStore, flowStateStore, previewArgs, initialPath } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let payloadData: any | undefined = undefined
	let pendingJson: string
	let addProperty: AddPropertyV2 | undefined = undefined
	let previewSchema: Record<string, any> | undefined = undefined
	let previewArguments: any | undefined = undefined
	let editOptionsOpen = false
	//const yOffset = 191

	const dropdownItems: Array<{
		label: string
		onClick: () => void
	}> = [
		{
			label: 'Json',
			onClick: () => {
				selectedTab = 'json'
				handleEditSchema(true)
			}
		},
		{
			label: "First step's inputs",
			onClick: () => {
				copyFirstStepSchema($flowStateStore, flowStore)
				handleEditSchema(false)
			}
		},
		{
			label: 'History',
			onClick: () => {
				selectedTab = 'history'
				handleEditSchema(true)
			}
		},
		{
			label: 'Saved Inputs',
			onClick: () => {
				selectedTab = 'savedInputs'
				handleEditSchema(true)
			}
		},
		{
			label: 'Captures',
			onClick: () => {
				selectedTab = 'captures'
				handleEditSchema(true)
			}
		}
	]

	let editSchema = false
	function handleEditSchema(edit?: boolean) {
		if (edit !== undefined) {
			editSchema = edit
		} else {
			editSchema = !editSchema
		}
	}

	function schemaFromPayload(payload: any) {
		const parsed = JSON.parse(JSON.stringify(payload))

		if (!parsed) {
			sendUserToast('Invalid JSON', true)
			return
		}

		return { required: [], properties: {}, ...convert(parsed) }
	}

	let flowPreviewContent: FlowPreviewContent
	let previewOpen = false

	let runDisabled = false
	function handleKeydown(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			runPreview()
		}
	}

	function runPreview() {
		if (!runDisabled) {
			previewOpen = true
			flowPreviewContent?.runPreview($previewArgs, undefined)
		}
	}

	async function updateRunDisabled(editSchema: boolean) {
		await tick()
		runDisabled = editSchema
	}
	$: updateRunDisabled(editSchema)

	let selectedTab: 'inputEditor' | 'history' | 'savedInputs' | 'json' | 'captures' = 'inputEditor'

	function updatePreviewSchema(payloadData: any) {
		if (!payloadData) {
			previewSchema = undefined
			previewArguments = undefined
			return
		}
		previewSchema = schemaFromPayload(payloadData)
		previewArguments = payloadData
	}

	$: updatePreviewSchema(payloadData)

	function applySchemaAndArgs() {
		if (!payloadData) {
			return
		}
		$flowStore.schema = previewSchema
		$previewArgs = previewArguments
		previousArgs = previewArguments
		payloadData = undefined
		editSchema = false
	}

	function applySchema() {
		if (!payloadData) return
		$flowStore.schema = schemaFromPayload(payloadData)
		editSchema = false
		previewSchema = undefined
		previewArguments = undefined
	}

	let previousArgs: Record<string, any> | undefined = undefined
	function updatePreviewArgs(previewArguments: Record<string, any>) {
		if (!previewArguments && previousArgs) {
			$previewArgs = previousArgs
		} else {
			previousArgs = $previewArgs
			$previewArgs = previewArguments
		}
	}
	$: updatePreviewArgs(previewArguments)

	let jsonValid = false
	function updatePayloadFromJson(pendingJson: string) {
		if (!pendingJson?.length) return
		try {
			const parsed = JSON.parse(pendingJson)
			payloadData = parsed
			jsonValid = true
		} catch (error) {
			jsonValid = false
		}
	}
	$: updatePayloadFromJson(pendingJson)
</script>

<!-- Add svelte:window to listen for keyboard events -->
<svelte:window on:keydown={handleKeydown} />

<Drawer bind:open={previewOpen} alwaysOpen size="75%">
	<FlowPreviewContent
		bind:this={flowPreviewContent}
		open={previewOpen}
		previewMode="whole"
		on:close={() => {
			previewOpen = false
		}}
	/>
</Drawer>

<FlowCard {noEditor} title="Flow Input">
	{#if !disabled}
		<div class="py-2 h-full">
			<EditableSchemaForm
				bind:schema={$flowStore.schema}
				isFlowInput
				on:edit={(e) => {
					addProperty?.openDrawer(e.detail)
				}}
				on:delete={(e) => {
					addProperty?.handleDeleteArgument([e.detail])
				}}
				offset={undefined}
				displayWebhookWarning
				{editSchema}
				{previewSchema}
				bind:args={$previewArgs}
				extraTab={selectedTab !== 'inputEditor'}
			>
				<svelte:fragment slot="openEditTab">
					<div
						class={twMerge(
							'flex flex-row divide-x border rounded-md bg-surface',
							editSchema ? 'rounded-r-none border-r-0' : ''
						)}
					>
						<button
							on:click={() => {
								selectedTab = 'inputEditor'
								handleEditSchema()
							}}
							class="hover:bg-surface-hover"
						>
							<div class="p-2 center-center">
								<svelte:component this={editSchema ? X : Pen} size={14} />
							</div>
						</button>
						<ButtonDropDown
							{dropdownItems}
							closeOnClick={true}
							bind:open={editOptionsOpen}
							placement="bottom-end"
						>
							<div class="p-2 center-center hover:bg-surface-hover">
								<ChevronDown size={16} />
							</div>
						</ButtonDropDown>
					</div>
				</svelte:fragment>
				<svelte:fragment slot="addProperty">
					{#if previewSchema && previewArguments}
						<div
							class={twMerge(
								'bg-blue-50 border-blue-200 border dark:bg-blue-900/40 dark:border-blue-700/40 text-xs py-2 w-full flex justify-center rounded-md',
								'text-blue-700 dark:text-blue-100'
							)}
						>
							<span> Preview only, apply to confirm</span>
						</div>
					{:else}
						<AddPropertyV2
							bind:schema={$flowStore.schema}
							bind:this={addProperty}
							on:change={() => {
								$flowStore = $flowStore
							}}
						>
							<svelte:fragment slot="trigger">
								<div
									class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
								>
									<Plus size={14} />
								</div>
							</svelte:fragment>
						</AddPropertyV2>
					{/if}
				</svelte:fragment>
				<svelte:fragment slot="extraTab">
					{#if selectedTab === 'history'}
						<FlowInputEditor
							name="From history"
							disabled={!payloadData}
							on:applySchemaAndArgs={applySchemaAndArgs}
							on:applySchema={applySchema}
						>
							<HistoricInpts
								scriptHash={null}
								scriptPath={null}
								flowPath={initialPath}
								on:select={(e) => {
									payloadData = e.detail
								}}
							/>
						</FlowInputEditor>
					{:else if selectedTab === 'captures'}
						<FlowInputEditor
							name="From captures"
							disabled={!payloadData}
							on:applySchemaAndArgs={applySchemaAndArgs}
							on:applySchema={applySchema}
						>
							<svelete:fragment slot="header">
								<CaptureButton on:openTriggers dark={true} small={true} />
							</svelete:fragment>
							<CapturesInputs
								on:select={(e) => {
									payloadData = e.detail
								}}
								scriptHash={null}
								flowPath={initialPath}
								isFlow={true}
							/>
						</FlowInputEditor>
					{:else if selectedTab === 'savedInputs'}
						<FlowInputEditor
							name="Saved inputs"
							disabled={!payloadData}
							on:applySchemaAndArgs={applySchemaAndArgs}
							on:applySchema={applySchema}
						>
							<svelete:fragment slot="header">
								<Tooltip>Shared inputs are available to anyone with access to the script</Tooltip>
							</svelete:fragment>
							<SavedInputsPickerV2
								flowPath={initialPath}
								on:select={(e) => {
									payloadData = e.detail
								}}
							/>
						</FlowInputEditor>
					{:else if selectedTab === 'json'}
						<FlowInputEditor
							name="From JSON"
							disabled={!pendingJson?.length || !jsonValid}
							on:applySchemaAndArgs={applySchemaAndArgs}
							on:applySchema={applySchema}
						>
							<SimpleEditor bind:code={pendingJson} lang="json" class="h-full" />
						</FlowInputEditor>
					{/if}
				</svelte:fragment>
				<svelte:fragment slot="runButton">
					<div class="w-full flex justify-end pr-5">
						<Button
							color="dark"
							btnClasses="w-fit"
							disabled={runDisabled}
							size="xs"
							shortCut={{ Icon: CornerDownLeft, hide: false }}
							on:click={() => {
								runPreview()
							}}
						>
							Run
						</Button>
					</div>
				</svelte:fragment>
			</EditableSchemaForm>
		</div>
	{:else}
		<div class="p-4 border-b">
			<FlowInputViewer schema={$flowStore.schema} />
		</div>
	{/if}
</FlowCard>
