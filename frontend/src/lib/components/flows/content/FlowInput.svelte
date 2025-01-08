<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ButtonType } from '$lib/components/common/button/model'
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
	import HistoricInputs from '$lib/components/HistoricInputs.svelte'
	import SavedInputsPicker from '$lib/components/SavedInputsPicker.svelte'
	import {
		CornerDownLeft,
		Pen,
		ChevronRight,
		ChevronDown,
		Plus,
		History,
		Braces,
		Code,
		Save,
		X
	} from 'lucide-svelte'
	import CaptureIcon from '$lib/components/triggers/CaptureIcon.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import FlowInputEditor from './FlowInputEditor.svelte'
	import CapturesInputs from '$lib/components/CapturesInputs.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'
	import ButtonDropDown from '$lib/components/meltComponents/ButtonDropDown.svelte'
	import CaptureButton from '$lib/components/triggers/CaptureButton.svelte'
	import RoundIconButton from '$lib/components/common/button/RoundIconButton.svelte'

	export let noEditor: boolean
	export let disabled: boolean

	const { flowStore, flowStateStore, previewArgs, pathStore, initialPath, flowInputEditorState } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let pendingJson: string
	let addProperty: AddPropertyV2 | undefined = undefined
	let previewSchema: Record<string, any> | undefined = undefined
	let previewArguments: Record<string, any> | undefined = $previewArgs
	let editOptionsOpen = false
	let dropdownItems: Array<{
		label: string
		onClick: () => void
		disabled?: boolean
	}> = []
	//const yOffset = 191

	let editPanelSize = $flowInputEditorState?.editPanelSize ?? 0
	function updateEditPanelSize(size: number | undefined) {
		if (!$flowInputEditorState) return
		if (!size || size === 0) {
			$flowInputEditorState.editPanelSize = undefined
			return
		}
		$flowInputEditorState.editPanelSize = size
	}
	$: updateEditPanelSize(editPanelSize)

	const getDropdownItems = () => {
		return [
			{
				label: 'Input editor',
				onClick: () => {
					handleEditSchema('inputEditor')
				},
				disabled:
					!$flowInputEditorState?.selectedTab ||
					$flowInputEditorState?.selectedTab === 'inputEditor',
				icon: Pen
			},
			{
				label: 'Trigger tests',
				onClick: () => {
					handleEditSchema('captures')
				},
				disabled: $flowInputEditorState?.selectedTab === 'captures',
				icon: CaptureIcon
			},
			{
				label: 'History',
				onClick: () => {
					handleEditSchema('history')
				},
				disabled: $flowInputEditorState?.selectedTab === 'history',
				icon: History
			},
			{
				label: 'Json',
				onClick: () => {
					handleEditSchema('json')
				},
				disabled: $flowInputEditorState?.selectedTab === 'json',
				icon: Braces
			},
			{
				label: "First step's inputs",
				onClick: () => {
					copyFirstStepSchema($flowStateStore, flowStore)
				},
				icon: Code
			},
			{
				label: 'Saved inputs',
				onClick: () => {
					handleEditSchema('savedInputs')
				},
				disabled: $flowInputEditorState?.selectedTab === 'savedInputs',
				icon: Save
			}
		].filter((item) => !item.disabled)
	}

	function handleEditSchema(editTab?: any) {
		if (!$flowInputEditorState) {
			return
		}
		if (editTab !== undefined) {
			$flowInputEditorState.selectedTab = editTab
		} else if ($flowInputEditorState.selectedTab === undefined) {
			$flowInputEditorState.selectedTab = 'inputEditor'
		} else {
			$flowInputEditorState.selectedTab = undefined
		}
	}

	function schemaFromPayload(payload: any) {
		const parsed = JSON.parse(JSON.stringify(payload))

		if (!parsed) {
			sendUserToast('Invalid Schema', true)
			return
		}

		if (Object.keys(parsed).length === 0) {
			return {
				required: [],
				properties: {},
				type: 'object',
				additionalProperties: false,
				order: []
			}
		}

		return { required: [], properties: {}, ...convert(parsed) }
	}

	let flowPreviewContent: FlowPreviewContent
	let previewOpen = false

	function handleKeydown(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			runPreview()
		}
	}

	function runPreview() {
		if (previewArguments) {
			$previewArgs = previewArguments
		}
		previewOpen = true
		flowPreviewContent?.test()
	}

	function updatePreviewSchemaAndArgs(payloadData: any) {
		if (!payloadData) {
			updatePreviewArguments(undefined)
			previewSchema = undefined
			return
		}
		previewSchema = schemaFromPayload(payloadData)
		updatePreviewArguments(payloadData)
	}
	$: updatePreviewSchemaAndArgs($flowInputEditorState.payloadData)

	function applySchemaAndArgs() {
		if (!previewSchema) {
			return
		}
		$flowStore.schema = previewSchema
		if (previewArguments) {
			$previewArgs = previewArguments
		}
		if ($flowInputEditorState) {
			$flowInputEditorState.selectedTab = undefined
		}
	}

	function applySchema() {
		$flowStore.schema = previewSchema
		if ($flowInputEditorState) {
			$flowInputEditorState.selectedTab = undefined
		}
	}

	function updatePreviewArguments(payloadData: Record<string, any> | undefined) {
		if (!payloadData) {
			previewArguments = $previewArgs
			return
		}
		previewArguments = payloadData
	}

	let jsonValid = false
	function updatePayloadFromJson(pendingJson: string) {
		if ($flowInputEditorState?.selectedTab !== 'json') {
			pendingJson = ''
			return
		}
		if (!pendingJson?.length) {
			$flowInputEditorState.payloadData = undefined
			updatePreviewSchemaAndArgs(undefined)
			jsonValid = false
			return
		}
		try {
			const parsed = JSON.parse(pendingJson)
			$flowInputEditorState.payloadData = parsed
			updatePreviewSchemaAndArgs(parsed)
			jsonValid = true
		} catch (error) {
			$flowInputEditorState.payloadData = undefined
			updatePreviewSchemaAndArgs(undefined)
			jsonValid = false
		}
	}
	$: updatePayloadFromJson(pendingJson)

	$: $flowInputEditorState, (dropdownItems = getDropdownItems())

	let tabButtonWidth = 0

	const TAB_TITLES: Record<string, string> = {
		inputEditor: 'Input editor',
		captures: 'Trigger tests',
		history: 'History',
		savedInputs: 'Saved inputs',
		json: 'JSON',
		undefined: ''
	}
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
		<div class="py-2 px-4 h-full">
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
				editTab={$flowInputEditorState?.selectedTab}
				{previewSchema}
				bind:args={previewArguments}
				bind:editPanelSize
				editPanelInitialSize={$flowInputEditorState?.editPanelSize}
				pannelExtraButtonWidth={$flowInputEditorState?.editPanelSize ? tabButtonWidth : 0}
			>
				<svelte:fragment slot="openEditTab">
					<div
						class={twMerge(
							'flex flex-row divide-x rounded-md bg-surface overflow-hidden',
							!!$flowInputEditorState?.selectedTab ? 'rounded-r-none' : '',
							ButtonType.ColorVariants.blue.divider
						)}
					>
						<button
							on:click={() => {
								handleEditSchema()
							}}
							title={!!$flowInputEditorState?.selectedTab
								? 'Close input editor'
								: 'Open input editor'}
							class={ButtonType.ColorVariants.blue.contained}
						>
							<div class="p-2 center-center">
								<svelte:component
									this={!!$flowInputEditorState?.selectedTab ? ChevronRight : Pen}
									size={14}
								/>
							</div>
						</button>

						<ButtonDropDown
							{dropdownItems}
							closeOnClick={true}
							bind:open={editOptionsOpen}
							placement="bottom-end"
						>
							<div
								class={twMerge(
									'p-2 center-center hover:bg-surface-hover',
									ButtonType.ColorVariants.blue.contained,
									'flex flex-row items-center gap-2 rounded-br-md',
									'transition-all duration-150 ease-in-out overflow-hidden whitespace-nowrap',
									!!$flowInputEditorState?.selectedTab ? 'w-[120px] px-3' : 'w-[30px]'
								)}
								bind:clientWidth={tabButtonWidth}
							>
								<div class="flex flex-row items-center gap-2 justify-between w-full">
									{#if !!$flowInputEditorState?.selectedTab}
										<h2 class="text-xs">{TAB_TITLES[$flowInputEditorState?.selectedTab]}</h2>
									{/if}
									<ChevronDown size={14} />
								</div>
							</div>
						</ButtonDropDown>
					</div>
				</svelte:fragment>
				<svelte:fragment slot="addProperty">
					{#if !!previewSchema}
						<div
							class={twMerge(
								'bg-blue-50 border-blue-200 border dark:bg-blue-900/40 dark:border-blue-700/40 text-xs p-2 w-full flex flex-row gap-2 items-center justify-center rounded-md',
								'text-blue-700 dark:text-blue-100',
								'relative'
							)}
						>
							<span> Preview only, save to update input schema</span>
							<div class="opacity-60"><CornerDownLeft size={14} /></div>
							<div class="flex flex-row gap-2 absolute right-2">
								<RoundIconButton><X size={12} /></RoundIconButton>
							</div>
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
					{#if $flowInputEditorState?.selectedTab === 'history'}
						<FlowInputEditor
							disabled={!$flowInputEditorState.payloadData}
							on:applySchemaAndArgs={applySchemaAndArgs}
							on:applySchema={applySchema}
							on:destroy={() => {
								$flowInputEditorState.payloadData = undefined
							}}
						>
							<HistoricInputs
								scriptHash={null}
								scriptPath={null}
								flowPath={$pathStore}
								on:select={(e) => {
									$flowInputEditorState.payloadData = e.detail ?? undefined
								}}
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'captures'}
						<FlowInputEditor
							disabled={!$flowInputEditorState.payloadData}
							on:applySchemaAndArgs={applySchemaAndArgs}
							on:applySchema={applySchema}
							on:destroy={() => {
								$flowInputEditorState.payloadData = undefined
							}}
						>
							<svelete:fragment slot="action">
								<div class="center-center">
									<CaptureButton on:openTriggers small={true} />
								</div>
							</svelete:fragment>
							<CapturesInputs
								on:select={(e) => {
									$flowInputEditorState.payloadData = e.detail ?? undefined
								}}
								flowPath={$pathStore}
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'savedInputs'}
						<FlowInputEditor
							disabled={!$flowInputEditorState.payloadData}
							on:applySchemaAndArgs={applySchemaAndArgs}
							on:applySchema={applySchema}
							on:destroy={() => {
								$flowInputEditorState.payloadData = undefined
							}}
						>
							<svelete:fragment slot="header">
								<Tooltip>Shared inputs are available to anyone with access to the script</Tooltip>
							</svelete:fragment>
							<SavedInputsPicker
								flowPath={initialPath}
								on:select={(e) => {
									$flowInputEditorState.payloadData = e.detail ?? undefined
								}}
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'json'}
						<FlowInputEditor
							disabled={!pendingJson?.length || !jsonValid}
							on:applySchemaAndArgs={applySchemaAndArgs}
							on:applySchema={applySchema}
							on:destroy={() => {
								$flowInputEditorState.payloadData = undefined
								pendingJson = ''
							}}
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
							disabled={!!previewSchema}
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
