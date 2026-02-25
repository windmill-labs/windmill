<script lang="ts">
	import type { AgentTool } from '../agentToolUtils'
	import {
		isFlowModuleTool,
		isMcpTool,
		isWebsearchTool,
		type McpTool
	} from '../agentToolUtils'
	import type { FlowModule } from '$lib/gen'
	import FlowModuleComponent from './FlowModuleComponent.svelte'
	import McpToolEditor from './McpToolEditor.svelte'
	import WebsearchToolDisplay from './WebsearchToolDisplay.svelte'

	interface Props {
		tool: AgentTool
		noEditor?: boolean
		enableAi?: boolean
		parentModule?: FlowModule | undefined
		previousModule?: FlowModule | undefined
		forceTestTab?: Record<string, boolean>
		highlightArg?: Record<string, string | undefined>
	}

	let {
		tool = $bindable(),
		noEditor = false,
		enableAi = false,
		parentModule = undefined,
		previousModule = undefined,
		forceTestTab,
		highlightArg
	}: Props = $props()
</script>

{#if isFlowModuleTool(tool)}
	<!-- FlowModule tool - use existing FlowModuleComponent -->
	<FlowModuleComponent
		{noEditor}
		flowModule={tool as FlowModule}
		{parentModule}
		{previousModule}
		failureModule={false}
		preprocessorModule={false}
		scriptKind="script"
		scriptTemplate="script"
		{enableAi}
		savedModule={undefined}
		forceTestTab={forceTestTab?.[tool.id]}
		highlightArg={highlightArg?.[tool.id]}
		isAgentTool={true}
	/>
{:else if isMcpTool(tool)}
	<!-- MCP tool - use McpToolEditor -->
	<McpToolEditor bind:tool={tool as McpTool} />
{:else if isWebsearchTool(tool)}
	<WebsearchToolDisplay />
{/if}
