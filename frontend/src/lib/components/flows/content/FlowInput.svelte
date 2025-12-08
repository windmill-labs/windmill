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
	import { isObjectTooBig, readFieldsRecursively } from '$lib/utils'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import type { AiAgent, ScriptLang } from '$lib/gen'
	import { deepEqual } from 'fast-equals'
	import Toggle from '$lib/components/Toggle.svelte'
	import { AI_AGENT_SCHEMA } from '../flowInfers'
	import { nextId } from '../flowModuleNextId'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import FlowChat from '../conversations/FlowChat.svelte'

	interface Props {
		noEditor: boolean
		disabled: boolean
		onTestFlow?: (conversationId?: string) => Promise<string | undefined>
		previewOpen: boolean
		flowModuleSchemaMap?: import('../map/FlowModuleSchemaMap.svelte').default
	}

	let {
		noEditor,
		disabled,
		onTestFlow,
		previewOpen,
		flowModuleSchemaMap = undefined
	}: Props = $props()
	const {
		flowStore,
		flowStateStore,
		previewArgs,
		pathStore,
		initialPathStore,
		fakeInitialPath,
		flowInputEditorState
	} = getContext<FlowEditorContext>('FlowEditorContext')

	// Get diffManager from the graph
	const diffManager = $derived(flowModuleSchemaMap?.getDiffManager())

	// Use pending schema from diffManager when in diff mode, otherwise use flowStore
	const effectiveSchema = $derived(diffManager?.currentInputSchema ?? flowStore.val.schema)

	let chatInputEnabled = $state(Boolean(flowStore.val.value?.chat_input_enabled))
	let shouldUseStreaming = $derived.by(() => {
		const modules = flowStore.val.value?.modules
		const lastModule = modules && modules.length > 0 ? modules[modules.length - 1] : undefined
		return (
			lastModule?.value?.type === 'aiagent' &&
			lastModule?.value?.input_transforms?.streaming?.type === 'static' &&
			lastModule?.value?.input_transforms?.streaming?.value === true
		)
	})
	let showChatModeWarning = $state(false)

	let addPropertyV2: AddPropertyV2 | undefined = $state(undefined)
	let previewSchema: Record<string, any> | undefined = $state(undefined)
	let payloadData: Record<string, any> | undefined = undefined
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
	let dynCode: string | undefined = $state(undefined)
	let dynLang: ScriptLang | undefined = $state(undefined)

	function updateEditPanelSize(size: number | undefined) {
		if (!$flowInputEditorState) return
		if (!size || size === 0) {
			$flowInputEditorState.editPanelSize = undefined
		} else {
			$flowInputEditorState.editPanelSize = size
		}
	}

	let timeout: number | undefined = undefined
	$effect(() => {
		if (timeout) {
			clearTimeout(timeout)
		}
		timeout = setTimeout(() => {
			updateEditPanelSize(editPanelSize)
		}, 100)
	})

	$effect(() => {
		chatInputEnabled = Boolean(flowStore.val.value?.chat_input_enabled)
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
			if (!previewOpen) {
				runPreview()
			}
		} else if (event.key === 'Enter' && previewSchema && !preventEnter) {
			applySchemaAndArgs()
			connectFirstNode()
			event.stopPropagation()
			event.preventDefault()
		}
	}

	async function runPreview() {
		await onTestFlow?.(undefined)
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
		if (previewArgs.val) {
			savedPreviewArgs = structuredClone($state.snapshot(previewArgs.val))
		}
		updatePreviewSchemaAndArgs(undefined)
		if ($flowInputEditorState) {
			$flowInputEditorState.selectedTab = undefined
		}
	}

	function updatePreviewArguments(payloadData: Record<string, any> | undefined) {
		if (!payloadData) {
			if (savedPreviewArgs) {
				previewArgs.val = savedPreviewArgs
			}
			return
		}
		savedPreviewArgs = structuredClone($state.snapshot(previewArgs.val))
		previewArgs.val = structuredClone($state.snapshot(payloadData))
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
			savedPreviewArgs = undefined
		}
	}

	$effect(() => {
		if (!previewArgs && savedPreviewArgs != undefined) {
			readFieldsRecursively(flowStore.val.schema)
			untrack(() => {
				resetArgs()
			})
		}
	})

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

	async function runFlowWithMessage(
		message: string,
		conversationId: string
	): Promise<string | undefined> {
		previewArgs.val = { user_message: message }
		const jobId = await onTestFlow?.(conversationId)
		return jobId
	}

	function hasOtherInputs(): boolean {
		const properties = flowStore.val.schema?.properties
		return Boolean(
			properties &&
				Object.keys(properties).length > 0 &&
				!(Object.keys(properties).length === 1 && Object.keys(properties).includes('user_message'))
		)
	}

	function handleToggleChatMode() {
		if (!flowStore.val.value?.chat_input_enabled) {
			// Check if there are existing inputs
			if (hasOtherInputs()) {
				showChatModeWarning = true
			} else {
				enableChatMode()
			}
		} else {
			// Disable chat input - remove from flow.value
			if (flowStore.val.value) {
				flowStore.val.value.chat_input_enabled = false
			}
		}
	}

	function enableChatMode() {
		// Enable chat input - set in flow.value
		flowStore.val.value.chat_input_enabled = true

		// Set up the schema for chat input
		flowStore.val.schema = {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			type: 'object',
			properties: {
				user_message: {
					type: 'string',
					description: 'Message from user'
				}
			},
			required: ['user_message']
		}

		// Find all AI agent modules
		const aiAgentModules = flowStore.val.value.modules.filter((m) => m.value.type === 'aiagent')

		if (aiAgentModules.length === 0) {
			// No AI agent exists, create one with context memory set to 10
			const aiAgentId = nextId(flowStateStore.val, flowStore.val)
			flowStore.val.value.modules = [
				...flowStore.val.value.modules,
				{
					id: aiAgentId,
					value: {
						type: 'aiagent',
						tools: [],
						input_transforms: Object.keys(AI_AGENT_SCHEMA.properties ?? {}).reduce(
							(accu, key) => {
								if (key === 'user_message') {
									accu[key] = { type: 'javascript', expr: 'flow_input.user_message' }
								} else if (key === 'messages_context_length') {
									accu[key] = { type: 'static', value: 10 }
								} else {
									accu[key] = {
										type: 'static',
										value: undefined
									}
								}
								return accu
							},
							{} as AiAgent['input_transforms']
						)
					}
				}
			]
			sendUserToast(
				'Chat mode enabled. AI agent created with user message input and context memory set to 10.',
				false
			)
		} else if (aiAgentModules.length === 1) {
			// Exactly one AI agent exists, configure it
			const aiAgent = aiAgentModules[0]
			const value = aiAgent.value as AiAgent

			// Set user_message to flow_input.user_message
			value.input_transforms['user_message'] = {
				type: 'javascript',
				expr: 'flow_input.user_message'
			}

			// Set messages_context_length to 10
			value.input_transforms['messages_context_length'] = {
				type: 'static',
				value: 10
			}

			sendUserToast(
				'Chat mode enabled. AI agent configured with user message input and context memory set to 10.',
				false
			)
		}
		// If there are multiple AI agents, don't auto-configure (ambiguous which one to configure)

		showChatModeWarning = false
	}
</script>

<!-- Add svelte:window to listen for keyboard events -->
<svelte:window onkeydown={handleKeydown} />

<ConfirmationModal
	open={showChatModeWarning}
	title="Enable Chat Mode?"
	confirmationText="Continue"
	onConfirmed={enableChatMode}
	onCanceled={() => {
		showChatModeWarning = false
		chatInputEnabled = false
	}}
>
	<p class="text-sm text-secondary">
		Enabling Chat Mode will replace all existing flow inputs with a single
		<span class="font-mono text-xs bg-surface-secondary px-1 rounded">user_message</span>
		parameter.
	</p>
	<p class="text-sm text-secondary mt-2">
		Your current input configuration will be lost. Are you sure you want to continue?
	</p>
</ConfirmationModal>

<FlowCard {noEditor} title="Flow Input">
	{#snippet action()}
		{#if !disabled}
			<Toggle
				size="sm"
				bind:checked={chatInputEnabled}
				on:change={(e) => {
					handleToggleChatMode()
				}}
				options={{
					right: 'Chat Mode',
					rightTooltip:
						'When enabled, the flow execution page will show a chat interface where each message sent runs the flow with the message as "user_message" input parameter. The flow schema will be automatically set to accept only a user_message string input.'
				}}
			/>
		{/if}
	{/snippet}
	{#if !disabled}
		<div class="flex flex-col h-full">
			{#if flowStore.val.value?.chat_input_enabled}
				<div class="flex flex-col h-full">
					<FlowChat
						onRunFlow={runFlowWithMessage}
						path={$pathStore}
						hideSidebar={true}
						useStreaming={shouldUseStreaming}
					/>
				</div>
			{:else}
				<div class="py-2 px-4 flex-1 min-h-0">
					<EditableSchemaForm
						bind:this={editableSchemaForm}
						bind:schema={flowStore.val.schema}
						isFlowInput
						on:delete={(e) => {
							addPropertyV2?.handleDeleteArgument([e.detail])
						}}
						showDynOpt
						displayWebhookWarning
						editTab={$flowInputEditorState?.selectedTab}
						{previewSchema}
						bind:args={previewArgs.val}
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
						onChange={() => {
							if (!previewSchema) {
								let args = $state.snapshot(previewArgs.val)
								if (!deepEqual(args, savedPreviewArgs)) {
									savedPreviewArgs = args
								}
							}
						}}
						bind:isValid
						bind:dynCode
						bind:dynLang
					>
						{#snippet openEditTab()}
							<div class={twMerge('flex flex-row divide-x', ButtonType.ColorVariants.blue.divider)}>
								<SideBarTab {dropdownItems} fullMenu={!!$flowInputEditorState?.selectedTab}>
									{#snippet close_button()}
										<Button
											onClick={() => handleEditSchema()}
											{...!!$flowInputEditorState?.selectedTab
												? {
														title: 'Close input editor',
														startIcon: { icon: ChevronRight },
														btnClasses: 'rounded-none rounded-tl-md'
													}
												: {
														title: 'Open input editor',
														startIcon: { icon: Pen }
													}}
											variant="accent"
											iconOnly
											wrapperClasses="h-full"
										/>
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
										variant="default"
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
									onAddNew={(argName) => {
										handleEditSchema('inputEditor')
										editableSchemaForm?.openField(argName)
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
										<div class="center-center">
											<CaptureButton on:openTriggers small={true} />
										</div>
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
										previewArgs={previewArgs.val}
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
										selected={!!previewArgs.val}
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
									variant="accent"
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
			{/if}
		</div>
	{:else}
		<div class="p-4 border-b">
			<FlowInputViewer schema={effectiveSchema} />
		</div>
	{/if}
</FlowCard>
