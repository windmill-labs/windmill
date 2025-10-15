<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		flowModule: FlowModule
		noEditor: boolean
	}

	let { flowModule = $bindable() }: Props = $props()

	interface McpToolInfo {
		name: string
		description?: string
		parameters: any
	}

	let summary = $state(flowModule.summary || '')
	let availableTools = $state<McpToolInfo[]>([])
	let loadingTools = $state(false)
	let toolsError = $state<string | undefined>(undefined)

	// Options for the multiselect
	let toolOptions = $derived(safeSelectItems(availableTools.map((t) => t.name)))

	$effect(() => {
		if (flowModule.value.type === 'mcpserver') {
			if (!flowModule.value.include_tools) {
				flowModule.value.include_tools = []
			}
			if (!flowModule.value.exclude_tools) {
				flowModule.value.exclude_tools = []
			}
		}
	})

	$effect(() => {
		if (flowModule.value.type === 'mcpserver') {
			refreshTools()
		}
	})

	async function refreshTools() {
		if (
			flowModule.value.type !== 'mcpserver' ||
			!flowModule.value.resource_path ||
			!$workspaceStore
		) {
			sendUserToast('Please specify a resource path first', true)
			return
		}

		loadingTools = true
		toolsError = undefined

		try {
			// Call the API to get MCP tools
			const tools = await ResourceService.getMcpTools({
				workspace: $workspaceStore,
				path: flowModule.value.resource_path
			})

			availableTools = tools
		} catch (error: any) {
			console.error('Failed to load MCP tools:', error)
			toolsError = error.body?.message || error.message || 'Failed to load tools from MCP server'
			if (toolsError) {
				sendUserToast(toolsError, true)
			}
			availableTools = []
		} finally {
			loadingTools = false
		}
	}
</script>

{#if flowModule.value.type === 'mcpserver'}
	<div class="flex flex-col gap-4 p-4">
		<!-- Summary Section -->
		<div class="w-full">
			<Label label="Summary">
				<input
					type="text"
					bind:value={summary}
					placeholder="e.g., GitHub API tools"
					class="text-sm w-full"
				/>
			</Label>
		</div>

		<!-- Resource Path Section -->
		<div class="w-full">
			<Label label="Resource Path">
				<input
					type="text"
					bind:value={flowModule.value.resource_path}
					placeholder="u/admin/my_mcp_server"
					class="text-sm w-full"
				/>
			</Label>
		</div>

		<!-- Available Tools Section -->
		<Section label="Available Tools">
			{#snippet action()}
				<Button
					size="xs"
					color="light"
					on:click={refreshTools}
					startIcon={{ icon: RefreshCw }}
					disabled={loadingTools}
				>
					{loadingTools ? 'Loading...' : 'Refresh Tools'}
				</Button>
			{/snippet}
			<div class="w-full flex flex-col gap-2">
				{#if toolsError}
					<div class="text-xs text-red-600 p-2 border border-red-300 rounded bg-red-50">
						{toolsError}
					</div>
				{/if}
				<div class="max-h-48 overflow-y-auto border rounded p-2 bg-surface-secondary">
					{#if loadingTools}
						<div class="text-xs text-secondary italic">Loading tools...</div>
					{:else if availableTools.length === 0}
						<div class="text-xs text-secondary italic">
							{toolsError
								? 'Failed to load tools. Please check the resource path and try again.'
								: 'No tools loaded yet. Click "Refresh Tools" to fetch tools from the MCP server.'}
						</div>
					{:else}
						<div class="flex flex-col gap-1">
							{#each availableTools as tool}
								<div class="text-xs">
									<span class="font-semibold">{tool.name}</span>
									{#if tool.description}
										<span class="text-secondary">— {tool.description}</span>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		</Section>

		<!-- Tool Filtering Section -->
		{#if flowModule.value.include_tools && flowModule.value.exclude_tools}
			<Section label="Tool Filtering">
				<div class="w-full flex flex-col gap-3">
					<div class="flex flex-col gap-2">
						<Label label="Select tools to include">
							<MultiSelect
								bind:value={flowModule.value.include_tools}
								items={toolOptions}
								placeholder="Choose tools to include..."
								disablePortal
							/>
						</Label>
					</div>
					<div class="flex flex-col gap-2">
						<Label label="Select tools to exclude">
							<MultiSelect
								bind:value={flowModule.value.exclude_tools}
								items={toolOptions}
								placeholder="Choose tools to exclude..."
								disablePortal
							/>
						</Label>
					</div>
				</div>
			</Section>
		{/if}
	</div>
{/if}
