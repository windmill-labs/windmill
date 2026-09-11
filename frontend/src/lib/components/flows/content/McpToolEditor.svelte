<script module lang="ts">
	import { get } from 'svelte/store'
	import { workspaceStore, userStore } from '$lib/stores'
	import { ResourceService } from '$lib/gen'
	import { createCache } from '$lib/utils'

	let loadToolsCached = createCache(
		({ workspace, path }: { workspace?: string; path?: string; refreshCount?: number }) =>
			workspace && path && get(userStore)
				? ResourceService.getMcpTools({ workspace, path })
				: undefined,
		{
			initial: { workspace: get(workspaceStore), path: undefined, refreshCount: 0 },
			invalidateMs: 1000 * 60
		}
	)
</script>

<script lang="ts">
	import type { McpTool } from '../agentToolUtils'
	import FlowCard from '../common/FlowCard.svelte'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { getContext, untrack } from 'svelte'
	import McpConnect from '$lib/components/mcp/McpConnect.svelte'
	import type { FlowEditorContext } from '../types'

	interface Props {
		tool: McpTool
		noEditor?: boolean
	}

	let { tool = $bindable(), noEditor = false }: Props = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	let opWs = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

	let refreshCount = $state(0)
	let resourcePicker: ResourcePicker | undefined = $state()

	let tools = usePromise(
		async () =>
			await loadToolsCached({
				workspace: opWs!,
				path: tool.value.resource_path,
				refreshCount
			}),
		{ loadInit: false, clearValueOnRefresh: false }
	)

	let toolOptions = $derived(safeSelectItems((tools.value ?? []).map((t) => t.name)))
	let resourcePath = $derived(tool.value.resource_path)
	let error = $derived(tools.error?.body?.message || tools.error?.message)

	$effect(() => {
		resourcePath
		opWs
		refreshCount
		untrack(() => {
			if (resourcePath?.length > 0) {
				tools.refresh()
			}
		})
	})

	$effect(() => {
		if (!tool.value.include_tools) {
			tool.value.include_tools = []
		}
		if (!tool.value.exclude_tools) {
			tool.value.exclude_tools = []
		}
	})

	$effect(() => {
		if (resourcePath?.length > 0 && tool.summary?.length === 0) {
			tool.summary = `MCP: ${tool.value.resource_path}`
		}
	})

	async function handleOAuthConnected(resourcePath: string, resourceName: string) {
		await resourcePicker?.refreshResources()
		tool.value.resource_path = resourcePath
		tool.summary = `MCP: ${resourceName}`
	}
</script>

<FlowCard {noEditor} title="MCP tool">
	<div class="flex flex-col gap-4 overflow-auto p-4" style="scrollbar-gutter: stable">
		<div class="w-full">
			<Label label="MCP resource">
				<ResourcePicker
					bind:this={resourcePicker}
					resourceType="mcp"
					placeholder="Select an MCP resource"
					bind:value={tool.value.resource_path}
					workspace={opWs}
				/>
			</Label>
		</div>

		{#if !resourcePath}
			<McpConnect
				workspace={opWs!}
				onConnected={(_ws, path) => handleOAuthConnected(path, path.split('/').pop() ?? path)}
			/>
		{/if}

		{#if resourcePath?.length > 0}
			<div class="w-full">
				<Label label="Summary">
					<input
						type="text"
						bind:value={tool.summary}
						placeholder="e.g., GitHub MCP"
						class="text-sm w-full"
					/>
				</Label>
			</div>

			<Section label="Available Tools">
				{#snippet action()}
					<Button
						size="xs"
						color="light"
						onClick={() => (refreshCount += 1)}
						startIcon={{ icon: RefreshCw }}
						disabled={tools.status === 'loading'}
					>
						{tools.status === 'loading' ? 'Loading...' : 'Refresh Tools'}
					</Button>
				{/snippet}
				<div class="w-full flex flex-col gap-2">
					{#if error}
						<div class="text-xs text-red-600 dark:text-red-400 mb-4"
							>{`Failed to load tools from MCP server: ${error}`}</div
						>
					{:else if tools.status === 'loading'}
						<div class="max-h-48 overflow-y-auto border rounded p-2 bg-surface-secondary">
							<div class="text-xs text-secondary italic">Loading tools...</div>
						</div>
					{:else if (tools.value ?? []).length === 0 && !error}
						<div class="max-h-48 overflow-y-auto border rounded p-2 bg-surface-secondary">
							<div class="text-xs text-secondary italic">
								No tools loaded yet. Click "Refresh Tools" to fetch tools from the MCP server.
							</div>
						</div>
					{:else if (tools.value ?? []).length > 0}
						<div class="max-h-48 overflow-y-auto border rounded p-2 bg-surface-secondary">
							<div class="flex flex-col gap-1">
								{#each tools.value ?? [] as mcpTool}
									<div class="text-xs">
										<span class="font-semibold">{mcpTool.name}</span>
										{#if mcpTool.description}
											<span class="text-secondary">— {mcpTool.description}</span>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			</Section>

			{#if tool.value.include_tools && tool.value.exclude_tools}
				<Section label="Tool Filtering">
					<div class="w-full flex flex-col gap-3">
						<div class="flex flex-col gap-2">
							<Label label="Only include specified tools">
								<MultiSelect
									bind:value={tool.value.include_tools}
									items={toolOptions}
									placeholder="Choose tools to include..."
								/>
							</Label>
						</div>
						<div class="flex flex-col gap-2">
							<Label label="Exclude specified tools">
								<MultiSelect
									bind:value={tool.value.exclude_tools}
									items={toolOptions}
									placeholder="Choose tools to exclude..."
								/>
							</Label>
						</div>
					</div>
				</Section>
			{/if}
		{/if}
	</div>
</FlowCard>
