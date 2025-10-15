<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'

	interface Props {
		flowModule: FlowModule
		noEditor: boolean
	}

	let { flowModule = $bindable() }: Props = $props()

	// Hardcoded tools for demo (TODO: Replace with API call)
	const HARDCODED_TOOLS = [
		{ name: 'filesystem_read', description: 'Read files from the filesystem' },
		{ name: 'filesystem_write', description: 'Write files to the filesystem' },
		{ name: 'filesystem_list', description: 'List directory contents' },
		{ name: 'database_query', description: 'Execute SQL queries' },
		{ name: 'database_insert', description: 'Insert data into database' },
		{ name: 'api_request', description: 'Make HTTP API requests' },
		{ name: 'search_web', description: 'Search the web for information' },
		{ name: 'send_email', description: 'Send emails via SMTP' }
	]

	let summary = $state(flowModule.summary || '')
	let resourcePath = $state<string>(
		flowModule.value.type === 'mcpserver' ? flowModule.value.resource_path : ''
	)
	let includeTools = $state<string[]>(
		flowModule.value.type === 'mcpserver' ? flowModule.value.include_tools || [] : []
	)
	let excludeTools = $state<string[]>(
		flowModule.value.type === 'mcpserver' ? flowModule.value.exclude_tools || [] : []
	)
	let availableTools = $state(HARDCODED_TOOLS)
	let loadingTools = $state(false)

	// Determine current filter mode
	let filterMode = $state<'include' | 'exclude' | 'none'>(
		includeTools.length > 0 ? 'include' : excludeTools.length > 0 ? 'exclude' : 'none'
	)

	// Options for the multiselect
	let toolOptions = $derived(safeSelectItems(availableTools.map((t) => t.name)))

	// Selected tools based on mode
	let selectedTools = $derived(filterMode === 'include' ? includeTools : excludeTools)

	// Watch for changes and sync back to flowModule
	$effect(() => {
		if (flowModule.value.type === 'mcpserver') {
			flowModule.summary = summary
			flowModule.value.resource_path = resourcePath
			flowModule.value.include_tools = includeTools
			flowModule.value.exclude_tools = excludeTools
		}
	})

	// When mode changes, reset selection
	function onModeChange(newMode: 'include' | 'exclude' | 'none') {
		filterMode = newMode
		selectedTools = []
	}

	function refreshTools() {
		loadingTools = true
		// Simulate API call
		setTimeout(() => {
			loadingTools = false
		}, 500)
	}
</script>

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
				bind:value={resourcePath}
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
			<div class="max-h-48 overflow-y-auto border rounded p-2 bg-surface-secondary">
				{#if loadingTools}
					<div class="text-xs text-secondary italic">Loading tools...</div>
				{:else if availableTools.length === 0}
					<div class="text-xs text-secondary italic">No tools available</div>
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
	<Section label="Tool Filtering">
		<div class="w-full flex flex-col gap-3">
			<div class="flex flex-col gap-2">
				<Label label="Filter Mode">
					<ToggleButtonGroup bind:selected={filterMode} onSelected={(value) => onModeChange(value)}>
						{#snippet children({ item })}
							<ToggleButton value="none" label="All Tools" {item} />
							<ToggleButton value="include" label="Include Only" {item} />
							<ToggleButton value="exclude" label="Exclude" {item} />
						{/snippet}
					</ToggleButtonGroup>
				</Label>

				{#if filterMode === 'none'}
					<div class="text-xs text-secondary">
						All tools from the MCP server will be available to the AI agent.
					</div>
				{:else if filterMode === 'include'}
					<div class="text-xs text-secondary">
						Only the selected tools will be available to the AI agent (whitelist).
					</div>
				{:else if filterMode === 'exclude'}
					<div class="text-xs text-secondary">
						All tools except the selected ones will be available (blacklist).
					</div>
				{/if}
			</div>

			{#if filterMode !== 'none'}
				<div class="flex flex-col gap-2">
					<Label
						label={filterMode === 'include' ? 'Select tools to include' : 'Select tools to exclude'}
					>
						<MultiSelect
							bind:value={selectedTools}
							items={toolOptions}
							placeholder={filterMode === 'include'
								? 'Choose tools to include...'
								: 'Choose tools to exclude...'}
							disablePortal
						/>
					</Label>

					{#if selectedTools.length > 0}
						<div class="text-xs text-secondary">
							{selectedTools.length} tool{selectedTools.length === 1 ? '' : 's'} selected
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</Section>
</div>
