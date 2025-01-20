<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ButtonType } from '$lib/components/common/button/model'
	import { getContext, tick } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
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
	import FirstStepInputs from '$lib/components/FirstStepInputs.svelte'
	import {
		CornerDownLeft,
		Pen,
		ChevronRight,
		Plus,
		History,
		Braces,
		Code,
		Save,
		X,
		Check
	} from 'lucide-svelte'
	import CaptureIcon from '$lib/components/triggers/CaptureIcon.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import FlowInputEditor from './FlowInputEditor.svelte'
	import CapturesInputs from '$lib/components/CapturesInputs.svelte'
	import { twMerge } from 'tailwind-merge'
	import CaptureButton from '$lib/components/triggers/CaptureButton.svelte'
	import {
		type SchemaDiff,
		getFullPath,
		setNestedProperty,
		getNestedProperty,
		schemaFromDiff,
		computeDiff,
		applyDiff
	} from '$lib/components/schema/schemaUtils'
	import SideBarTab from '$lib/components/meltComponents/SideBarTab.svelte'

	export let noEditor: boolean
	export let disabled: boolean

	const { flowStore, previewArgs, pathStore, initialPath, flowInputEditorState } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let pendingJson: string
	let addProperty: AddPropertyV2 | undefined = undefined
	let previewSchema: Record<string, any> | undefined = undefined
	let payloadData: Record<string, any> | undefined = undefined
	let previewArguments: Record<string, any> | undefined = $previewArgs
	let dropdownItems: Array<{
		label: string
		onClick: () => void
		disabled?: boolean
	}> = []
	let diff: Record<string, SchemaDiff> = {}
	let editPanelSize = $flowInputEditorState?.editPanelSize ?? 0
	let selectedSchema: Record<string, any> | undefined = undefined
	let runDisabled: boolean = false
	let editableSchemaForm: EditableSchemaForm | undefined = undefined

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
				icon: Pen,
				selected: $flowInputEditorState?.selectedTab === 'inputEditor'
			},
			{
				label: 'Trigger captures',
				onClick: () => {
					handleEditSchema('captures')
				},
				disabled: $flowInputEditorState?.selectedTab === 'captures',
				icon: CaptureIcon,
				selected: $flowInputEditorState?.selectedTab === 'captures'
			},
			{
				label: 'History',
				onClick: () => {
					handleEditSchema('history')
				},
				disabled: $flowInputEditorState?.selectedTab === 'history',
				icon: History,
				selected: $flowInputEditorState?.selectedTab === 'history'
			},
			{
				label: 'Json payload',
				onClick: () => {
					handleEditSchema('json')
				},
				disabled: $flowInputEditorState?.selectedTab === 'json',
				icon: Braces,
				selected: $flowInputEditorState?.selectedTab === 'json'
			},
			{
				label: "First step's inputs",
				onClick: () => {
					handleEditSchema('firstStepInputs')
				},
				icon: Code,
				selected: $flowInputEditorState?.selectedTab === 'firstStepInputs'
			},
			{
				label: 'Saved inputs',
				onClick: () => {
					handleEditSchema('savedInputs')
				},
				disabled: $flowInputEditorState?.selectedTab === 'savedInputs',
				icon: Save,
				selected: $flowInputEditorState?.selectedTab === 'savedInputs'
			}
		]
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

	function schemaFromPayload(payloadData: any) {
		const payload = structuredClone(payloadData)
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
		} else if (event.key === 'Enter' && previewSchema && !preventEnter) {
			applySchemaAndArgs()
			connectFirstNode()
			event.stopPropagation()
			event.preventDefault()
		}
	}

	function runPreview() {
		if (previewArguments) {
			$previewArgs = previewArguments
		}
		previewOpen = true
		flowPreviewContent?.test()
	}

	function updatePreviewSchemaAndArgs(payload: any) {
		if (!payload) {
			payloadData = undefined
			selectedSchema = undefined
			updatePreviewArguments(undefined)
			updatePreviewSchema(undefined)
			return
		}
		payloadData = structuredClone(payload)
		selectedSchema = schemaFromPayload(payloadData)
		updatePreviewSchema(selectedSchema)
		updatePreviewArguments(payloadData)
	}

	async function updatePreviewSchema(newSchema: Record<string, any> | undefined) {
		if (!newSchema) {
			previewSchema = undefined
			if (runDisabled) {
				await tick()
				runDisabled = false
			}
			diff = {}
			return
		}
		diff = {}
		const diffSchema = computeDiff(newSchema, $flowStore.schema)
		diff = diffSchema
		previewSchema = schemaFromDiff(diffSchema, $flowStore.schema)
		runDisabled = true
	}

	async function applySchemaAndArgs() {
		$flowStore.schema = applyDiff($flowStore.schema, diff)
		if (previewArguments) {
			$previewArgs = structuredClone(previewArguments)
		}
		updatePreviewSchemaAndArgs(undefined)
		if ($flowInputEditorState) {
			$flowInputEditorState.selectedTab = undefined
		}
	}

	function updatePreviewArguments(payloadData: Record<string, any> | undefined) {
		if (!payloadData) {
			previewArguments = structuredClone($previewArgs)
			return
		}
		previewArguments = structuredClone(payloadData)
	}

	function updatePayloadFromJson(jsonInput: string) {
		if (jsonInput === undefined || jsonInput === null || jsonInput.trim() === '') {
			updatePreviewSchemaAndArgs(undefined)
			return
		}
		try {
			const parsed = JSON.parse(jsonInput)
			updatePreviewSchemaAndArgs(parsed)
		} catch (error) {
			updatePreviewSchemaAndArgs(undefined)
		}
	}

	let tabButtonWidth = 0

	let connectFirstNode: () => void = () => {}

	let init = false
	function initPayloadData() {
		if ($flowInputEditorState.payloadData && !init) {
			init = true
			updatePreviewSchemaAndArgs($flowInputEditorState.payloadData)
			$flowInputEditorState.payloadData = undefined
		}
	}
	$: $flowInputEditorState && ((dropdownItems = getDropdownItems()), initPayloadData())

	let preventEnter = false

	async function acceptChange(arg: { label: string; nestedParent: any | undefined }) {
		handleChange(arg, $flowStore.schema, diff, (newSchema) => {
			$flowStore.schema = newSchema
		})
	}

	async function rejectChange(arg: { label: string; nestedParent: any | undefined }) {
		const revertDiff = computeDiff($flowStore.schema, selectedSchema)
		handleChange(arg, selectedSchema, revertDiff, (newSchema) => {
			selectedSchema = newSchema
		})
	}

	function handleChange(
		arg: { label: string; nestedParent: any | undefined },
		currentSchema: Record<string, any> | undefined,
		diffSchema: Record<string, SchemaDiff> | undefined,
		updateCurrentSchema: (newSchema: Record<string, any> | undefined) => void
	) {
		if (!diff || !currentSchema) {
			return
		}

		const path = getFullPath(arg)
		const parentPath = path.slice(0, -1)
		const diffStatus = getNestedProperty({ diff: diffSchema }, path, 'diff')
		const schema = getNestedProperty(currentSchema, parentPath, 'properties')
		const localDiff = {
			[arg.label]: diffStatus
		}
		const schemaUpdated = applyDiff(schema, localDiff)
		if (parentPath.length > 0) {
			setNestedProperty(currentSchema, parentPath, schemaUpdated)
		} else {
			updateCurrentSchema(schemaUpdated)
		}

		diff = computeDiff(selectedSchema, $flowStore.schema)
		previewSchema = schemaFromDiff(diff, $flowStore.schema)
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
				bind:this={editableSchemaForm}
				bind:schema={$flowStore.schema}
				isFlowInput
				on:edit={(e) => {
					addProperty?.openDrawer(e.detail)
				}}
				on:delete={(e) => {
					addProperty?.handleDeleteArgument([e.detail])
				}}
				displayWebhookWarning
				editTab={$flowInputEditorState?.selectedTab}
				{previewSchema}
				bind:args={previewArguments}
				bind:editPanelSize
				editPanelInitialSize={$flowInputEditorState?.editPanelSize}
				pannelExtraButtonWidth={$flowInputEditorState?.editPanelSize ? tabButtonWidth : 0}
				{diff}
				disableDnd={!!previewSchema}
				on:rejectChange={(e) => {
					rejectChange(e.detail).then(() => {
						updatePreviewSchema(selectedSchema)
					})
				}}
				on:acceptChange={(e) => {
					acceptChange(e.detail).then(() => {
						updatePreviewSchema(selectedSchema)
					})
				}}
			>
				<svelte:fragment slot="openEditTab">
					<div class={twMerge('flex flex-row divide-x', ButtonType.ColorVariants.blue.divider)}>
						<SideBarTab {dropdownItems} fullMenu={!!$flowInputEditorState?.selectedTab}>
							<svelte:fragment slot="close button">
								<button
									on:click={() => {
										handleEditSchema()
									}}
									title={!!$flowInputEditorState?.selectedTab
										? 'Close input editor'
										: 'Open input editor'}
									class={twMerge(
										ButtonType.ColorVariants.blue.contained,
										!!$flowInputEditorState?.selectedTab
											? 'rounded-tl-md border-l border-t'
											: 'rounded-md border'
									)}
								>
									<div class="p-2 center-center">
										<svelte:component
											this={!!$flowInputEditorState?.selectedTab ? ChevronRight : Pen}
											size={14}
										/>
									</div>
								</button>
							</svelte:fragment>
						</SideBarTab>
					</div>
				</svelte:fragment>
				<svelte:fragment slot="addProperty">
					{#if !!previewSchema}
						<div class="flex flex-row items-center gap-2 right-2 justify-end">
							<Button
								size="xs"
								color="green"
								disabled={!previewSchema}
								shortCut={{ Icon: CornerDownLeft, hide: false, withoutModifier: true }}
								startIcon={{ icon: Check }}
								on:click={() => {
									applySchemaAndArgs()
									connectFirstNode()
								}}
							>
								Update schema
							</Button>
							<Button
								variant="border"
								color="light"
								size="xs"
								startIcon={{ icon: X }}
								shortCut={{ key: 'esc', withoutModifier: true }}
								nonCaptureEvent
							/>
						</div>
					{:else}
						<AddPropertyV2
							bind:schema={$flowStore.schema}
							bind:this={addProperty}
							on:change={() => {
								$flowStore = $flowStore
								if (editableSchemaForm) {
									editableSchemaForm.updateJson()
								}
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
							title="History"
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
							}}
						>
							<HistoricInputs
								scriptHash={null}
								scriptPath={null}
								flowPath={$pathStore}
								on:select={(e) => {
									updatePreviewSchemaAndArgs(e.detail ?? undefined)
								}}
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'captures'}
						<FlowInputEditor
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
							}}
							title="Trigger captures"
						>
							<svelete:fragment slot="action">
								<div class="center-center">
									<CaptureButton on:openTriggers small={true} />
								</div>
							</svelete:fragment>
							<CapturesInputs
								on:select={(e) => {
									updatePreviewSchemaAndArgs(e.detail ?? undefined)
								}}
								flowPath={$pathStore}
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'savedInputs'}
						<FlowInputEditor
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
							}}
							title="Saved inputs"
						>
							<SavedInputsPicker
								flowPath={initialPath}
								on:select={(e) => {
									updatePreviewSchemaAndArgs(e.detail ?? undefined)
								}}
								on:isEditing={(e) => {
									preventEnter = e.detail
								}}
								previewArgs={previewArguments}
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'json'}
						<FlowInputEditor
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
							}}
							title="Json payload"
						>
							<SimpleEditor
								on:focus={() => {
									preventEnter = true
									updatePayloadFromJson(pendingJson)
								}}
								on:blur={async () => {
									preventEnter = false
									setTimeout(() => {
										if (payloadData) {
											updatePayloadFromJson('')
										}
									}, 100)
								}}
								on:change={(e) => {
									updatePayloadFromJson(e.detail.code)
								}}
								bind:code={pendingJson}
								lang="json"
								class="h-full"
								placeholder={'Write a JSON payload. The input schema will be inferred.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}'}
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'firstStepInputs'}
						<FlowInputEditor
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
								connectFirstNode = () => {}
							}}
							title="First step's inputs"
						>
							<FirstStepInputs
								on:connectFirstNode={({ detail }) => {
									connectFirstNode = detail.connectFirstNode
								}}
								on:select={(e) => {
									if (e.detail) {
										const diffSchema = computeDiff(e.detail, $flowStore.schema)
										diff = diffSchema
										previewSchema = schemaFromDiff(diffSchema, $flowStore.schema)
										runDisabled = true
									} else {
										updatePreviewSchemaAndArgs(undefined)
									}
								}}
							/>
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
