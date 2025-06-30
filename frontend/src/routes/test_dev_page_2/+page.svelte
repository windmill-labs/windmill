<script lang="ts">
	// import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	// aiChatManager

	// import type { AIProviderModel, ScriptLang } from '$lib/gen/types.gen'
	// import type { ScriptOptions } from './ContextManager.svelte'
	// import {
	// 	flowTools,
	// 	prepareFlowSystemMessage,
	// 	prepareFlowUserMessage,
	// 	type FlowAIChatHelpers
	// } from './flow/core'
	// import ContextManager from './ContextManager.svelte'
	// import HistoryManager from './HistoryManager.svelte'
	// import { processToolCall, type DisplayMessage, type Tool, type ToolCallbacks } from './shared'

	// HistoryManager
	// ContextManager
	// import type {
	// 	ChatCompletionChunk,
	// 	ChatCompletionMessageParam,
	// 	ChatCompletionMessageToolCall,
	// 	ChatCompletionSystemMessageParam,
	// 	ChatCompletionUserMessageParam
	// } from 'openai/resources/chat/completions.mjs'

	// import { sendUserToast } from '$lib/toast'
	// import { dfs } from '$lib/components/flows/previousResults'
	// import type { FlowModuleState, FlowState } from '$lib/components/flows/flowState'
	// import type { CurrentEditor, ExtendedOpenFlow } from '$lib/components/flows/types'
	// import { untrack } from 'svelte'
	// import type { DBSchemas } from '$lib/stores'
	// import { getCompletion } from '$lib/components/copilot/lib'
	// import { getStringError } from '$lib/components/copilot/chat/utils'
	// import {
	// 	navigatorTools,
	// 	prepareNavigatorSystemMessage,
	// 	prepareNavigatorUserMessage
	// } from '$lib/components/copilot/chat/navigator/core'
	// import {
	// 	prepareScriptSystemMessage,
	// 	prepareScriptTools,
	// 	prepareScriptUserMessage
	// } from '$lib/components/copilot/chat/script/core'
	// import { loadApiTools } from '$lib/components/copilot/chat/navigator/apiTools'
	// import { langToExt } from '$lib/editorUtils'
	// import { langToExt } from '$lib/editorLangUtils'

	// import { ResourceService, type ListResourceResponse, type ScriptLang } from '$lib/gen'
	// import { scriptLangToEditorLang } from '$lib/scripts'

	// import HistoryManager from '$lib/components/copilot/chat/HistoryManager.svelte'
	// import ContextManager from '$lib/components/copilot/chat/ContextManager.svelte'
	// loadApiTools
	// import { askTools, prepareAskSystemMessage } from './ask/core'
	// import { chatState, DEFAULT_SIZE, triggerablesByAi } from './sharedChatState.svelte'

	// import FlowStatusViewerInner from '$lib/components/FlowStatusViewerInner.svelte'

	// import ArgTest from '$lib/components/ArgTest.svelte'
	// import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'

	// AppPrevi
	// import ArgInput from '$lib/components/ArgInput.svelte'

	// import SchemaForm from '$lib/components/SchemaForm.svelte'

	// import RunnableComponent from '$lib/components/apps/components/helpers/RunnableComponent.svelte'

	// import ComponentInner from '$lib/components/apps/editor/component/ComponentInner.svelte'

	// import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	// import { getContext, onDestroy, setContext, untrack } from 'svelte'
	// import { get, writable, type Writable } from 'svelte/store'
	// import { buildWorld } from '../rx'
	// import type {
	// 	App,
	// 	AppViewerContext,
	// 	ConnectingInput,
	// 	EditorBreakpoint,
	// 	EditorMode
	// } from '../types'
	// import type { Policy } from '$lib/gen'
	// import { Unlock } from 'lucide-svelte'
	// import GridViewer from '$lib/components/apps/editor/GridViewer.svelte'
	// import { twMerge } from 'tailwind-merge'
	// import { ButtonType } from '$lib/components/common'

	// import { dfs, maxHeight } from './appUtils'
	// import { BG_PREFIX, migrateApp } from '../utils'
	// import { workspaceStore, enterpriseLicense } from '$lib/stores'
	// import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	// import { getTheme } from './componentsPanel/themeUtils'
	// import HiddenComponent from '../components/helpers/HiddenComponent.svelte'
	// import RecomputeAllComponents from './RecomputeAllComponents.svelte'

	// HiddenComponent,
	// RecomputeAllComponents
	// import HiddenComponent from '$lib/components/apps/components/helpers/HiddenComponent.svelte'
	// RunnableComponent
	// import { buildWorld } from '../rx'
	// import type {
	// 	App,
	// 	AppViewerContext,
	// 	ConnectingInput,
	// 	EditorBreakpoint,
	// 	EditorMode
	// } from '../types'
	// import type { Policy } from '$lib/gen'
	// import Button from '../../common/button/Button.svelte'
	// import { Unlock } from 'lucide-svelte'
	// import GridViewer from './GridViewer.svelte'
	// import Component from './component/Component.svelte'
	// import { twMerge } from 'tailwind-merge'
	// import { deepEqual } from 'fast-equals'
	// import { dfs, maxHeight } from './appUtils'
	// import { BG_PREFIX, migrateApp } from '../utils'
	// import { workspaceStore, enterpriseLicense } from '$lib/stores'
	// import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	// import { getTheme } from './componentsPanel/themeUtils'
	// import HiddenComponent from '../components/helpers/HiddenComponent.svelte'
	// import RecomputeAllComponents from './RecomputeAllComponents.svelte'
	// ComponentInner
	// import { Button } from '$lib/components/common'

	// SchemaForm

	// ArgInput

	// import {
	// 	setInputCat as computeInputCat,
	// 	debounce,
	// 	emptyString,
	// 	getSchemaFromProperties
	// } from '$lib/utils'
	// import { DollarSign, Pipette, Plus, X, Check, Loader2 } from 'lucide-svelte'
	// import { createEventDispatcher, onDestroy, onMount, tick, untrack } from 'svelte'
	// import { fade } from 'svelte/transition'
	// import { Button, SecondsInput } from './common'
	// import FieldHeader from './FieldHeader.svelte'
	// import type ItemPicker from './ItemPicker.svelte'
	// import ObjectResourceInput from './ObjectResourceInput.svelte'
	// import Range from './Range.svelte'
	// import ResourcePicker from './ResourcePicker.svelte'
	// import type SimpleEditor from './SimpleEditor.svelte'
	// import Toggle from './Toggle.svelte'
	// import type VariableEditor from './VariableEditor.svelte'
	// import { twMerge } from 'tailwind-merge'
	// import ArgEnum from './ArgEnum.svelte'
	// import DateTimeInput from './DateTimeInput.svelte'
	// import DateInput from './DateInput.svelte'
	// import S3FilePicker from './S3FilePicker.svelte'
	// import CurrencyInput from './apps/components/inputs/currency/CurrencyInput.svelte'
	// import FileUpload from './common/fileUpload/FileUpload.svelte'
	// import autosize from '$lib/autosize'
	// import PasswordArgInput from './PasswordArgInput.svelte'
	// import Password from './Password.svelte'
	// import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	// import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	// import SchemaFormDnd from '$lib/components/schema/SchemaFormDND.svelte'
	// import SchemaForm from './SchemaForm.svelte'
	// import { deepEqual } from 'fast-equals'
	// import DynSelect from './DynSelect.svelte'
	// import type { Script } from '$lib/gen'
	// import type { SchemaDiff } from '$lib/components/schema/schemaUtils.svelte'
	// import type { ComponentCustomCSS } from './apps/types'
	// import MultiSelect from './select/MultiSelect.svelte'
	// import { safeSelectItems } from './select/utils.svelte'
	// SchemaFormDnd

	// 	import FlowJobResult from './FlowJobResult.svelte'
	// import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	// import DisplayResult from './DisplayResult.svelte'

	// FlowJobResult
	// import FlowJobResult from '$lib/components/FlowJobResult.svelte'
	// import FlowPreviewStatus from '$lib/components/preview/FlowPreviewStatus.svelte'
	// import DisplayResult from '$lib/components/DisplayResult.svelte'

	// FlowJobResult
	// FlowPreviewStatus
	// DisplayResult

	// FlowStatusViewerInner

	// import JobArgs from './JobArgs.svelte'
	// import { ChevronDown, Hourglass, Loader2 } from 'lucide-svelte'
	// import FlowStatusWaitingForEvents from './FlowStatusWaitingForEvents.svelte'
	// import { deepEqual } from 'fast-equals'
	// import FlowTimeline from './FlowTimeline.svelte'
	// import { dfs } from './flows/dfs'
	// import { get, writable, type Unsubscriber, type Writable } from 'svelte/store'
	// import Alert from './common/alert/Alert.svelte'
	// import FlowGraphViewerStep from './FlowGraphViewerStep.svelte'
	// import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	// import { buildPrefix } from './graph/graphBuilder.svelte'

	// import JobArgs from '$lib/components/JobArgs.svelte'
	// import FlowStatusWaitingForEvents from '$lib/components/FlowStatusWaitingForEvents.svelte'

	// import DisplayResult from '$lib/components/DisplayResult.svelte'
	// import FlowTimeline from '$lib/components/FlowTimeline.svelte'
	// import { dfs } from '$lib/components/flows/dfs'
	// import { get, writable, type Unsubscriber, type Writable } from 'svelte/store'

	// TimelineBar

	// import Alert from '$lib/components/common/alert/Alert.svelte'
	// import FlowGraphViewerStep from '$lib/components/FlowGraphViewerStep.svelte'
	// import FlowGraphV2 from '$lib/components/graph/FlowGraphV2.svelte'
	// import { buildPrefix } from '$lib/components/graph/graphBuilder.svelte'
	// import Popover from '$lib/components/Popover.svelte'
	// import TimelineBar from '$lib/components/TimelineBar.svelte'
	// Popover

	// import ModuleNode from './renderers/nodes/ModuleNode.svelte'
	// import InputNode from './renderers/nodes/InputNode.svelte'
	// import BranchAllStart from './renderers/nodes/BranchAllStart.svelte'
	// import BranchAllEndNode from './renderers/nodes/BranchAllEndNode.svelte'
	// import ForLoopEndNode from './renderers/nodes/ForLoopEndNode.svelte'
	// import ForLoopStartNode from './renderers/nodes/ForLoopStartNode.svelte'
	// import ResultNode from './renderers/nodes/ResultNode.svelte'
	// import BaseEdge from './renderers/edges/BaseEdge.svelte'
	// import EmptyEdge from './renderers/edges/EmptyEdge.svelte'
	// import { sugiyama, dagStratify, coordCenter, decrossTwoLayer, decrossOpt } from 'd3-dag'
	// import { Expand } from 'lucide-svelte'
	// import Toggle from '../Toggle.svelte'
	// import DataflowEdge from './renderers/edges/DataflowEdge.svelte'
	// import { encodeState } from '$lib/utils'
	// import BranchOneStart from './renderers/nodes/BranchOneStart.svelte'
	// import NoBranchNode from './renderers/nodes/NoBranchNode.svelte'
	// import HiddenBaseEdge from './renderers/edges/HiddenBaseEdge.svelte'
	// import TriggersNode from './renderers/nodes/TriggersNode.svelte'
	// import { Alert, Drawer } from '../common'
	// import Button from '../common/button/Button.svelte'
	// import FlowYamlEditor from '../flows/header/FlowYamlEditor.svelte'
	// import BranchOneEndNode from './renderers/nodes/branchOneEndNode.svelte'
	// import type { TriggerContext } from '../triggers'
	// import { workspaceStore } from '$lib/stores'
	// import SubflowBound from './renderers/nodes/SubflowBound.svelte'
	// import { deepEqual } from 'fast-equals'
	// import ViewportResizer from './ViewportResizer.svelte'

	// import ModuleNode from '$lib/components/graph/renderers/nodes/ModuleNode.svelte'
	// import InputNode from '$lib/components/graph/renderers/nodes/InputNode.svelte'
	// import BranchAllStart from '$lib/components/graph/renderers/nodes/BranchAllStart.svelte'
	// import BranchAllEndNode from '$lib/components/graph/renderers/nodes/BranchAllEndNode.svelte'
	// import ForLoopEndNode from '$lib/components/graph/renderers/nodes/ForLoopEndNode.svelte'
	// import ForLoopStartNode from '$lib/components/graph/renderers/nodes/ForLoopStartNode.svelte'
	// import ResultNode from '$lib/components/graph/renderers/nodes/ResultNode.svelte'
	// import BaseEdge from '$lib/components/graph/renderers/edges/BaseEdge.svelte'
	// import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
</script>
