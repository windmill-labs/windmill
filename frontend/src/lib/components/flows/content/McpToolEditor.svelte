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
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { untrack } from 'svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import McpOAuthConnect from './McpOAuthConnect.svelte'

	interface Props {
		tool: McpTool
	}

	let { tool = $bindable() }: Props = $props()

	let showOAuthForm = $state(false)
	let refreshCount = $state(0)

	let tools = usePromise(
		async () =>
			await loadToolsCached({
				workspace: $workspaceStore!,
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
		$workspaceStore
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

	function handleOAuthConnected(resourcePath: string, resourceName: string) {
		tool.value.resource_path = resourcePath
		tool.summary = `MCP: ${resourceName}`
		showOAuthForm = false
	}
</script>

<div class="flex flex-col gap-4 p-4">
	<Alert type="info" title="MCP Client Configuration">
		{#snippet children()}
			<p class="mb-2 text-sm">
				MCP clients allow AI agents to access and execute a list of tools made available by an MCP
				server.
				<br />
				Choose an MCP resource to make its tools available to the agent.
				<br />
				<br />
				<strong>Note:</strong> Only HTTP streamable MCP servers are supported.
			</p>
		{/snippet}
	</Alert>

	<div class="w-full">
		<Label label="MCP Resource">
			<ResourcePicker resourceType="mcp" bind:value={tool.value.resource_path} />
		</Label>
	</div>

	{#if !resourcePath}
		{#if !showOAuthForm}
			<Button size="xs" color="light" onClick={() => (showOAuthForm = true)}>
				Connect with OAuth
			</Button>
		{:else}
			<McpOAuthConnect
				onConnected={handleOAuthConnected}
				onCancel={() => (showOAuthForm = false)}
			/>
		{/if}
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
