<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ButtonType } from '$lib/components/common/button/model'
	import { getContext, tick, untrack } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import JsonInputs from '$lib/components/JsonInputs.svelte'
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
	import FlowInputEditor from './FlowInputEditor.svelte'
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
	} from '$lib/components/schema/schemaUtils.svelte'
	import SideBarTab from '$lib/components/meltComponents/SideBarTab.svelte'
	import CaptureTable from '$lib/components/triggers/CaptureTable.svelte'
	import { isObjectTooBig, DynamicSelect } from '$lib/utils'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'

	interface Props {
		noEditor: boolean
		disabled: boolean
		onTestFlow?: () => void
	}

	let { noEditor, disabled, onTestFlow }: Props = $props()

	const {
		flowStore,
		previewArgs,
		pathStore,
		initialPathStore,
		fakeInitialPath,
		flowInputEditorState
	} = getContext<FlowEditorContext>('FlowEditorContext')

	let addPropertyV2: AddPropertyV2 | undefined = $state(undefined)
	let previewSchema: Record<string, any> | undefined = $state(undefined)
	let payloadData: Record<string, any> | undefined = undefined
	let previewArguments: Record<string, any> | undefined = $state(previewArgs.val)
	let dropdownItems: Array<{
		label: string
		onClick: () => void
		disabled?: boolean
	}> = $state([])
	let diff: Record<string, SchemaDiff> = $state({})
	let editPanelSize = $state($flowInputEditorState?.editPanelSize ?? 0)
	let selectedSchema: Record<string, any> | undefined = $state(undefined)
	let runDisabled: boolean = $state(false)
	let editableSchemaForm: EditableSchemaForm | undefined = $state(undefined)
	let savedPreviewArgs: Record<string, any> | undefined = $state(undefined)
	let isValid = $state(true)

	$effect.pre(() => {
		if (!flowStore.val.dynselect_code) {
			flowStore.val.dynselect_code = DynamicSelect.DEFAULT_DYNSELECT_TYPESCRIPT
		}
		if (!flowStore.val.dynselect_lang) {
			flowStore.val.dynselect_lang = 'bun'
		}
	})

	function updateEditPanelSize(size: number | undefined) {
		if (!$flowInputEditorState) return
		if (!size || size === 0) {
			$flowInputEditorState.editPanelSize = undefined
		} else {
			$flowInputEditorState.editPanelSize = size
		}
	}

	let timeout: NodeJS.Timeout | undefined = undefined
	$effect(() => {
		if (timeout) {
			clearTimeout(timeout)
		}
		timeout = setTimeout(() => {
			updateEditPanelSize(editPanelSize)
		}, 100)
	})

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
		const payload = structuredClone($state.snapshot(payloadData))
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
			previewArgs.val = structuredClone($state.snapshot(previewArguments))
		}
		onTestFlow?.()
	}

	function updatePreviewSchemaAndArgs(payload: any) {
		if (isObjectTooBig(payload)) {
			sendUserToast('Payload too big to be used as input', true)
			return
		}
		if (!payload) {
			payloadData = undefined
			selectedSchema = undefined
			updatePreviewArguments(undefined)
			updatePreviewSchema(undefined)
			return
		}
		payloadData = structuredClone($state.snapshot(payload))
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
		const diffSchema = computeDiff(newSchema, flowStore.val.schema)
		diff = diffSchema
		previewSchema = schemaFromDiff(diffSchema, flowStore.val.schema)
		runDisabled = true
	}

	async function applySchemaAndArgs() {
		flowStore.val.schema = applyDiff(flowStore.val.schema, diff)
		if (previewArguments) {
			savedPreviewArgs = structuredClone($state.snapshot(previewArguments))
		}
		updatePreviewSchemaAndArgs(undefined)
		if ($flowInputEditorState) {
			$flowInputEditorState.selectedTab = undefined
		}
	}

	function updatePreviewArguments(payloadData: Record<string, any> | undefined) {
		if (!payloadData) {
			previewArguments = savedPreviewArgs
			return
		}
		savedPreviewArgs = structuredClone($state.snapshot(previewArguments))
		previewArguments = structuredClone($state.snapshot(payloadData))
	}

	let tabButtonWidth = 0

	let connectFirstNode: () => void = $state(() => {})

	let init = false
	function initPayloadData() {
		if ($flowInputEditorState.payloadData && !init) {
			init = true
			updatePreviewSchemaAndArgs($flowInputEditorState.payloadData)
			$flowInputEditorState.payloadData = undefined
		}
	}
	$effect(() => {
		if ($flowInputEditorState) {
			untrack(() => {
				dropdownItems = getDropdownItems()
				initPayloadData()
			})
		}
	})

	let preventEnter = $state(false)

	async function acceptChange(arg: { label: string; nestedParent: any | undefined }) {
		handleChange(arg, flowStore.val.schema, diff, (newSchema) => {
			flowStore.val.schema = newSchema
		})
	}

	async function rejectChange(arg: { label: string; nestedParent: any | undefined }) {
		const revertDiff = computeDiff(flowStore.val.schema, selectedSchema)
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

		diff = computeDiff(selectedSchema, flowStore.val.schema)
		previewSchema = schemaFromDiff(diff, flowStore.val.schema)
	}

	function resetArgs() {
		if (!previewSchema) {
			// previewArguments = undefined
			savedPreviewArgs = undefined
		}
	}

	let historicInputs: HistoricInputs | undefined = $state(undefined)
	let captureTable: CaptureTable | undefined = $state(undefined)
	let savedInputsPicker: SavedInputsPicker | undefined = $state(undefined)
	let jsonInputs: JsonInputs | undefined = $state(undefined)
	let firstStepInputs: FirstStepInputs | undefined = $state(undefined)

	function resetSelected() {
		historicInputs?.resetSelected(true)
		captureTable?.resetSelected(true)
		savedInputsPicker?.resetSelected(true)
		jsonInputs?.resetSelected(true)
		firstStepInputs?.resetSelected(true)
	}
</script>

<!-- Add svelte:window to listen for keyboard events -->
<svelte:window onkeydown={handleKeydown} />

<FlowCard {noEditor} title="Flow Input">
	{#if !disabled}
		<div class="py-2 px-4 h-full">
			<EditableSchemaForm
				bind:this={editableSchemaForm}
				bind:schema={flowStore.val.schema}
				isFlowInput
				on:edit={(e) => {
					addPropertyV2?.openDrawer(e.detail)
				}}
				on:delete={(e) => {
					addPropertyV2?.handleDeleteArgument([e.detail])
				}}
				displayWebhookWarning
				editTab={$flowInputEditorState?.selectedTab}
				{previewSchema}
				bind:args={previewArguments}
				bind:editPanelSize={
					() => {
						return editPanelSize
					},
					(v) => {
						if (editPanelSize != v) {
							editPanelSize = v
						}
					}
				}
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
				shouldDispatchChanges={true}
				on:change={() => {
					previewArguments = previewArguments
					if (!previewSchema) {
						savedPreviewArgs = structuredClone($state.snapshot(previewArguments))
					}
					refreshStateStore(flowStore)
				}}
				on:schemaChange={() => {
					resetArgs()
				}}
				bind:isValid
				bind:dynSelectCode={flowStore.val.dynselect_code}
				bind:dynSelectLang={flowStore.val.dynselect_lang}
			>
				{#snippet openEditTab()}
					<div class={twMerge('flex flex-row divide-x', ButtonType.ColorVariants.blue.divider)}>
						<SideBarTab {dropdownItems} fullMenu={!!$flowInputEditorState?.selectedTab}>
							{#snippet close_button()}
								<button
									onclick={() => {
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
										{#if !!$flowInputEditorState?.selectedTab}
											<ChevronRight size={14} />
										{:else}
											<Pen size={14} />
										{/if}
									</div>
								</button>
							{/snippet}
						</SideBarTab>
					</div>
				{/snippet}
				{#snippet addProperty()}
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
								{Object.values(diff).every((el) => el.diff === 'same')
									? 'Apply args'
									: 'Update schema'}
							</Button>
							<Button
								variant="border"
								color="light"
								size="xs"
								startIcon={{ icon: X }}
								shortCut={{ key: 'esc', withoutModifier: true }}
								on:click={() => {
									resetSelected()
								}}
							/>
						</div>
					{:else}
						<AddPropertyV2
							bind:schema={flowStore.val.schema}
							bind:this={addPropertyV2}
							on:change={() => {
								refreshStateStore(flowStore)
								if (editableSchemaForm) {
									editableSchemaForm.updateJson()
								}
							}}
							on:addNew={(e) => {
								handleEditSchema('inputEditor')
								editableSchemaForm?.openField(e.detail)
								refreshStateStore(flowStore)
							}}
						>
							{#snippet trigger()}
								<div
									class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
									id="add-flow-input-btn"
								>
									<Plus size={14} />
								</div>
							{/snippet}
						</AddPropertyV2>
					{/if}
				{/snippet}
				{#snippet extraTab()}
					{#if $flowInputEditorState?.selectedTab === 'history'}
						<FlowInputEditor
							title="History"
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
							}}
						>
							<HistoricInputs
								bind:this={historicInputs}
								runnableId={$initialPathStore ?? undefined}
								runnableType={$pathStore ? 'FlowPath' : undefined}
								on:select={(e) => {
									updatePreviewSchemaAndArgs(e.detail?.args ?? undefined)
								}}
								limitPayloadSize
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'captures'}
						<FlowInputEditor
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
							}}
							title="Trigger captures"
						>
							{#snippet action()}
								<svelete:fragment>
									<div class="center-center">
										<CaptureButton on:openTriggers small={true} />
									</div>
								</svelete:fragment>
							{/snippet}
							<div class="h-full">
								<CaptureTable
									path={$initialPathStore || fakeInitialPath}
									on:select={(e) => {
										updatePreviewSchemaAndArgs(e.detail ?? undefined)
									}}
									isFlow={true}
									headless={true}
									addButton={false}
									bind:this={captureTable}
									limitPayloadSize
								/>
							</div>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'savedInputs'}
						<FlowInputEditor
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
							}}
							title="Saved inputs"
						>
							<SavedInputsPicker
								runnableId={$initialPathStore ?? undefined}
								runnableType={$pathStore ? 'FlowPath' : undefined}
								on:select={(e) => {
									updatePreviewSchemaAndArgs(e.detail ?? undefined)
								}}
								on:isEditing={(e) => {
									preventEnter = e.detail
								}}
								previewArgs={previewArguments}
								{isValid}
								limitPayloadSize
								bind:this={savedInputsPicker}
							/>
						</FlowInputEditor>
					{:else if $flowInputEditorState?.selectedTab === 'json'}
						<FlowInputEditor
							on:destroy={() => {
								updatePreviewSchemaAndArgs(undefined)
							}}
							title="Json payload"
						>
							<JsonInputs
								on:focus={() => {
									preventEnter = true
								}}
								on:blur={async () => {
									preventEnter = false
								}}
								on:select={(e) => {
									updatePreviewSchemaAndArgs(e.detail ?? undefined)
								}}
								selected={!!previewArguments}
								bind:this={jsonInputs}
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
								bind:this={firstStepInputs}
								on:connectFirstNode={({ detail }) => {
									connectFirstNode = detail.connectFirstNode
								}}
								on:select={(e) => {
									if (e.detail) {
										const diffSchema = computeDiff(e.detail, flowStore.val.schema)
										diff = diffSchema
										previewSchema = schemaFromDiff(diffSchema, flowStore.val.schema)
										runDisabled = true
									} else {
										updatePreviewSchemaAndArgs(undefined)
									}
								}}
							/>
						</FlowInputEditor>
					{/if}
				{/snippet}
				{#snippet runButton()}
					<div class="w-full flex justify-end pr-5">
						<Button
							color="dark"
							btnClasses="w-fit"
							disabled={runDisabled || !isValid}
							size="xs"
							shortCut={{ Icon: CornerDownLeft, hide: false }}
							on:click={() => {
								runPreview()
							}}
						>
							Run
						</Button>
					</div>
				{/snippet}
			</EditableSchemaForm>
		</div>
	{:else}
		<div class="p-4 border-b">
			<FlowInputViewer schema={flowStore.val.schema} />
		</div>
	{/if}
</FlowCard>
