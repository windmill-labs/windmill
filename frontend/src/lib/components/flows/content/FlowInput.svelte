<script lang="ts">
	import { Button } from '$lib/components/common'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import { ButtonType } from '$lib/components/common/button/model'
	import { getContext, tick, untrack } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import JsonInputs from '$lib/components/JsonInputs.svelte'
	import { convert } from '@redocly/json-to-json-schema'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
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
		Check,
		Settings2,
		MessageCircle
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
	import type { AiAgent, InputTransform, ScriptLang } from '$lib/gen'
	import { deepEqual } from 'fast-equals'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { AI_AGENT_SCHEMA } from '../flowInfers'
	import { agentStreamingEnabled } from '../agentFormFields'
	import { nextId } from '../flowModuleNextId'
	import FlowChat from '../conversations/FlowChat.svelte'
	import { isEmptyAgentChatInputValue } from '../conversations/agentChatInputs'
	import { SPECIAL_MODULE_IDS } from '$lib/components/copilot/chat/shared'

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
		flowInputEditorState,
		opWorkspace
	} = getContext<FlowEditorContext>('FlowEditorContext')
	// Acting workspace when the flow editor runs in an AI session; else the nav workspace.
	let opWs = $derived(opWorkspace?.() ?? $workspaceStore)

	// Get diffManager from the graph
	const diffManager = $derived(flowModuleSchemaMap?.getDiffManager())

	// Use pending schema from diffManager when in diff mode, otherwise use flowStore
	const effectiveSchema = $derived(diffManager?.currentInputSchema ?? flowStore.val.schema)

	// Detect if we're in "review mode" (AI has made schema changes that are pending)
	const hasAiSchemaChanges = $derived(
		Boolean(
			diffManager?.moduleActions[SPECIAL_MODULE_IDS.INPUT]?.pending &&
				diffManager?.beforeFlow?.schema
		)
	)

	let chatInputEnabled = $state(Boolean(flowStore.val.value?.chat_input_enabled))
	let shouldUseStreaming = $derived.by(() => {
		const modules = flowStore.val.value?.modules
		const lastModule = modules && modules.length > 0 ? modules[modules.length - 1] : undefined
		if (lastModule?.value?.type !== 'aiagent') return false
		return agentStreamingEnabled(lastModule.value)
	})
	// Chat mode shows one of the two at a time: the conversation, or the inputs it sends.
	let chatPanelTab = $state<'chat' | 'inputs'>('chat')
	let chatInputsEditTab = $state(false)
	let chatEditableSchemaForm: EditableSchemaForm | undefined = $state(undefined)
	let chatInputsAddPropertyV2: AddPropertyV2 | undefined = $state(undefined)

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

	// Set up review mode when AI has made schema changes that are pending
	$effect(() => {
		hasAiSchemaChanges
		diffManager?.beforeFlow?.schema
		flowStore.val.schema
		untrack(() => {
			if (hasAiSchemaChanges) {
				// In review mode, selectedSchema = beforeSchema (what we might revert to)
				selectedSchema = structuredClone($state.snapshot(diffManager!.beforeFlow!.schema))
				diff = computeDiff(flowStore.val.schema, selectedSchema)
				previewSchema = schemaFromDiff(diff, selectedSchema)
				runDisabled = true
				if (Object.values(diff).every((d) => d.diff === 'same')) {
					diffManager?.acceptModule(SPECIAL_MODULE_IDS.INPUT, flowStore)
				}
			}
		})
	})

	$effect(() => {
		if (!hasAiSchemaChanges) {
			selectedSchema = undefined
			previewSchema = undefined
			diff = {}
			runDisabled = false
		}
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
		if (hasAiSchemaChanges) {
			// Review mode: Accept all AI changes
			diffManager?.acceptModule(SPECIAL_MODULE_IDS.INPUT, flowStore)
		} else {
			// Preview mode: Apply the diff to flowStore
			flowStore.val.schema = applyDiff(flowStore.val.schema, diff)
			if (previewArgs.val) {
				savedPreviewArgs = structuredClone($state.snapshot(previewArgs.val))
			}
			updatePreviewSchemaAndArgs(undefined)
			if ($flowInputEditorState) {
				$flowInputEditorState.selectedTab = undefined
			}
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
		if (hasAiSchemaChanges) {
			// Review mode: Accept = finalize the AI change (keep current value)
			handleChangeInReviewMode(arg, 'accept')
		} else {
			// Preview mode: Accept = apply the change to flowStore
			handleChange(arg, flowStore.val.schema, diff, (newSchema) => {
				flowStore.val.schema = newSchema
			})
		}
	}

	async function rejectChange(arg: { label: string; nestedParent: any | undefined }) {
		if (hasAiSchemaChanges) {
			// Review mode: Reject = revert flowStore field to beforeSchema value
			handleChangeInReviewMode(arg, 'reject')
		} else {
			// Preview mode: Reject = remove proposal from selectedSchema
			const revertDiff = computeDiff(flowStore.val.schema, selectedSchema)
			handleChange(arg, selectedSchema, revertDiff, (newSchema) => {
				selectedSchema = newSchema
			})
		}
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

	function handleChangeInReviewMode(
		arg: { label: string; nestedParent: any | undefined },
		action: 'accept' | 'reject'
	) {
		// Accept: source=flowStore.val.schema (current), target=beforeSchema
		// Reject: source=beforeSchema, target=flowStore.val.schema (current)
		const beforeSchema = diffManager?.beforeFlow?.schema
		const sourceSchema = action === 'accept' ? flowStore.val.schema : beforeSchema
		const targetSchema = action === 'accept' ? beforeSchema : flowStore.val.schema

		if (!beforeSchema || !sourceSchema || !targetSchema) return

		const path = getFullPath(arg)
		const parentPath = path.slice(0, -1)

		const getSchemaAtPath = (schema: Record<string, any>) =>
			parentPath.length === 0 ? schema : getNestedProperty(schema, parentPath, 'properties')

		const getProperties = (schema: Record<string, any>) => getSchemaAtPath(schema)?.properties

		const sourceProperties = getProperties(sourceSchema)
		const targetProperties = getProperties(targetSchema)
		const targetSchemaAtPath = getSchemaAtPath(targetSchema)
		const sourceValue = sourceProperties?.[arg.label]

		if (sourceValue !== undefined) {
			if (targetProperties) {
				targetProperties[arg.label] = structuredClone($state.snapshot(sourceValue))
				if (targetSchemaAtPath?.order && !targetSchemaAtPath.order.includes(arg.label)) {
					targetSchemaAtPath.order.push(arg.label)
				}
			}
		} else {
			if (targetProperties && arg.label in targetProperties) {
				delete targetProperties[arg.label]
				if (targetSchemaAtPath?.order) {
					targetSchemaAtPath.order = targetSchemaAtPath.order.filter((x: string) => x !== arg.label)
				}
			}
		}

		if (action === 'accept') {
			diffManager.beforeFlow.schema = { ...beforeSchema }
		} else {
			flowStore.val.schema = { ...flowStore.val.schema }
		}
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
		conversationId: string,
		additionalInputs?: Record<string, any>
	): Promise<string | undefined> {
		previewArgs.val = {
			user_message: message,
			...(additionalInputs ?? {})
		}
		const jobId = await onTestFlow?.(conversationId)
		return jobId
	}

	function handleToggleChatMode() {
		if (!flowStore.val.value?.chat_input_enabled) {
			enableChatMode()
		} else {
			// Disable chat input - remove from flow.value
			if (flowStore.val.value) {
				flowStore.val.value.chat_input_enabled = false
			}
		}
	}

	/**
	 * Add the flow input the agent's `user_attachments` reads, and return its name. Chat
	 * mode means files dropped in the composer, and that only works through a flow input —
	 * so it is set up with the message and the memory rather than left to be discovered.
	 */
	function addAttachmentsInput(): string {
		const schema = (flowStore.val.schema ?? {}) as Record<string, any>
		const properties: Record<string, any> = (schema.properties ??= {})
		let name = 'files'
		for (let i = 2; name in properties; i++) name = `files_${i}`
		properties[name] = {
			type: 'array',
			items: { type: 'object', resourceType: 's3object' },
			description: 'Images or PDFs for the agent to read'
		}
		flowStore.val.schema = schema
		return name
	}

	function enableChatMode() {
		// Enable chat input - set in flow.value
		flowStore.val.value.chat_input_enabled = true

		// The chat fills the flow's form rather than standing in for it: all it needs is a
		// `user_message` string, which the server requires under that exact argument name.
		// Every other input stays — the composer drives the ones an agent field reads, and
		// the rest are asked for in the Configure-inputs modal.
		const schema = flowStore.val.schema ?? {}
		const properties = { ...(schema.properties ?? {}) }
		// Only a string can carry the message; anything else here cannot be what chat sends.
		if (properties['user_message']?.type !== 'string') {
			properties['user_message'] = { type: 'string', description: 'Message from user' }
		}
		const required: string[] = Array.isArray(schema.required) ? schema.required : []
		flowStore.val.schema = {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			...schema,
			type: 'object',
			properties,
			required: required.includes('user_message') ? required : [...required, 'user_message']
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
								} else if (key === 'user_attachments') {
									accu[key] = { type: 'javascript', expr: `flow_input.${addAttachmentsInput()}` }
								} else if (key === 'memory') {
									accu[key] = { type: 'static', value: { kind: 'auto', context_length: 10 } }
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
				'Chat mode enabled. AI agent created with user message and attachments inputs, and context memory set to 10.',
				false
			)
		} else if (aiAgentModules.length === 1) {
			// Exactly one AI agent exists: fill in defaults only for inputs the
			// user hasn't configured, so re-enabling chat mode on an already
			// configured agent doesn't clobber a custom user_message expression.
			const aiAgent = aiAgentModules[0]
			const value = aiAgent.value as AiAgent

			// Degenerate shapes the input form can produce without deliberate
			// configuration count as unconfigured: empty static value (undefined
			// persists as null through JSON round-trips, and the step panel seeds an
			// array-typed field with []), blank JS expression (the JS toggle seeds a
			// bare backtick pair), or an AI transform (meaningless for the chat input).
			const isUnconfigured = (transform: InputTransform | undefined) =>
				transform === undefined ||
				(transform.type === 'static' && isEmptyAgentChatInputValue(transform.value)) ||
				(transform.type === 'javascript' && transform.expr.replaceAll('`', '').trim() === '') ||
				transform.type === 'ai'

			const applied: string[] = []
			if (isUnconfigured(value.input_transforms['user_message'])) {
				value.input_transforms['user_message'] = {
					type: 'javascript',
					expr: 'flow_input.user_message'
				}
				applied.push('user message input')
			}

			if (isUnconfigured(value.input_transforms['user_attachments'])) {
				value.input_transforms['user_attachments'] = {
					type: 'javascript',
					expr: `flow_input.${addAttachmentsInput()}`
				}
				applied.push('attachments input')
			}

			// `off` is the first oneOf variant of the memory field, so a step added by hand
			// carries it without anyone choosing it — and an agent that forgets every turn
			// makes the chat a series of unrelated questions. Overwritten rather than left
			// alone; the toast below says it happened.
			const memoryIsOff = (transform: InputTransform | undefined) =>
				transform?.type === 'static' && (transform.value as any)?.kind === 'off'

			if (
				isUnconfigured(value.input_transforms['memory']) ||
				memoryIsOff(value.input_transforms['memory'])
			) {
				value.input_transforms['memory'] = {
					type: 'static',
					value: { kind: 'auto', context_length: 10 }
				}
				applied.push('context memory set to 10')
			}

			// Without streaming the chat has no SSE to read, so a turn shows nothing —
			// no thinking, no answer — until the run ends and its rows are written.
			if (
				isUnconfigured(value.input_transforms['streaming']) ||
				(value.input_transforms['streaming']?.type === 'static' &&
					value.input_transforms['streaming'].value === false)
			) {
				value.input_transforms['streaming'] = { type: 'static', value: true }
				applied.push('streaming turned on')
			}

			sendUserToast(
				applied.length > 0
					? `Chat mode enabled. AI agent configured with ${applied.join(' and ')}.`
					: 'Chat mode enabled. Existing AI agent configuration kept unchanged.',
				false
			)
		}
		// If there are multiple AI agents, don't auto-configure (ambiguous which one to configure)
	}
</script>

<!-- Add svelte:window to listen for keyboard events -->
<svelte:window onkeydown={handleKeydown} />

<!-- The edit toggle and the add-input target, shared by both panels below: chat mode
     carries a smaller set of side tabs, but the controls themselves must not differ. -->
{#snippet inputsEditButton(open: boolean, toggle: () => void)}
	<Button
		onClick={toggle}
		{...open
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

{#snippet inputsAddTrigger()}
	<div
		class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
		id="add-flow-input-btn"
	>
		<Plus size={14} />
	</div>
{/snippet}

<FlowCard {noEditor} title="Flow Input">
	{#snippet action()}
		{#if !disabled}
			<div class="flex items-center gap-2">
				<Toggle
					size="xs"
					bind:checked={chatInputEnabled}
					on:change={() => {
						handleToggleChatMode()
					}}
					options={{
						right: 'Chat Mode',
						rightTooltip:
							'Turns this flow\'s page into a chat. Each message runs the flow with the message as its "user_message" input, and is kept as a chat — one conversation per chat, each with its own AI agent memory. Chats started in the editor are marked as tests and stay out of the deployed flow\'s list.',
						rightDocumentationLink:
							'https://www.windmill.dev/docs/core_concepts/ai_agents#chat-mode'
					}}
				/>
				{#if flowStore.val.value?.chat_input_enabled}
					<ToggleButtonGroup bind:selected={chatPanelTab} noWFull>
						{#snippet children({ item })}
							<ToggleButton size="sm" value="chat" label="Chat" icon={MessageCircle} {item} />
							<ToggleButton
								size="sm"
								value="inputs"
								label="Inputs"
								icon={Settings2}
								tooltip="Edit the flow inputs the chat sends alongside each message"
								{item}
							/>
						{/snippet}
					</ToggleButtonGroup>
				{/if}
			</div>
		{/if}
	{/snippet}
	{#if !disabled}
		<div class="flex flex-col h-full">
			{#if flowStore.val.value?.chat_input_enabled}
				<div class="flex flex-col h-full">
					{#if chatPanelTab === 'inputs'}
						<!-- EditableSchemaForm scrolls internally against `h-full`, so the wrapper has
						     to be bounded (flex-1 min-h-0) or the form grows to content height and
						     spills out of the panel. -->
						<div class="py-2 px-4 flex-1 min-h-0">
							<EditableSchemaForm
								bind:this={chatEditableSchemaForm}
								bind:schema={flowStore.val.schema}
								lockedArgs={['user_message']}
								isFlowInput
								showSensitiveToggle
								workspace={opWs}
								editTab={chatInputsEditTab ? 'inputEditor' : undefined}
								showDynOpt
								bind:dynCode
								bind:dynLang
								on:delete={(e) => {
									chatInputsAddPropertyV2?.handleDeleteArgument([e.detail])
								}}
							>
								{#snippet openEditTab()}
									{@render inputsEditButton(
										chatInputsEditTab,
										() => (chatInputsEditTab = !chatInputsEditTab)
									)}
								{/snippet}
								{#snippet addProperty()}
									<AddPropertyV2
										bind:this={chatInputsAddPropertyV2}
										bind:schema={flowStore.val.schema}
										onAddNew={(argName) => {
											chatInputsEditTab = true
											chatEditableSchemaForm?.openField(argName)
											refreshStateStore(flowStore)
										}}
									>
										{#snippet trigger()}
											{@render inputsAddTrigger()}
										{/snippet}
									</AddPropertyV2>
								{/snippet}
							</EditableSchemaForm>
						</div>
					{:else}
						<FlowChat
							onRunFlow={runFlowWithMessage}
							conversationKind="test"
							path={$pathStore}
							useStreaming={shouldUseStreaming}
							inputSchema={flowStore.val.schema}
							flowModules={flowStore.val.value?.modules}
						/>
					{/if}
				</div>
			{:else}
				<div class="py-2 px-4 flex-1 min-h-0">
					<EditableSchemaForm
						bind:this={editableSchemaForm}
						bind:schema={flowStore.val.schema}
						isFlowInput
						showSensitiveToggle
						workspace={opWs}
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
							const isAiChange = hasAiSchemaChanges
							rejectChange(e.detail).then(() => {
								if (!isAiChange) {
									updatePreviewSchema(selectedSchema)
								}
							})
						}}
						on:acceptChange={(e) => {
							acceptChange(e.detail).then(() => {
								const isAiChange = hasAiSchemaChanges
								if (!isAiChange) {
									updatePreviewSchema(selectedSchema)
								}
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
										{@render inputsEditButton(!!$flowInputEditorState?.selectedTab, () =>
											handleEditSchema()
										)}
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
										onClick={() => {
											applySchemaAndArgs()
											connectFirstNode()
										}}
									>
										{#if hasAiSchemaChanges}
											Accept all changes
										{:else if Object.values(diff).every((el) => el.diff === 'same')}
											Apply args
										{:else}
											Update schema
										{/if}
									</Button>
									<Button
										variant="default"
										size="xs"
										startIcon={{ icon: X }}
										shortCut={{ key: 'esc', withoutModifier: true }}
										onClick={() => {
											if (hasAiSchemaChanges) {
												if (diffManager?.beforeFlow?.schema) {
													diffManager.rejectModule(SPECIAL_MODULE_IDS.INPUT, flowStore)
												}
											} else {
												resetSelected()
											}
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
										{@render inputsAddTrigger()}
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
										workspace={opWs}
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
											workspace={opWs}
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
										workspace={opWs}
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
									onClick={() => {
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
