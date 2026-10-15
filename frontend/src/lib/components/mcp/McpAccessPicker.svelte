<script lang="ts" module>
	export type McpAccess = 'read' | 'read_run' | 'full' | 'selected'

	const ACCESS_SCOPES: Record<Exclude<McpAccess, 'selected'>, string[]> = {
		read: ['mcp:all', 'mcp:read_only'],
		read_run: ['mcp:all', 'mcp:read_run'],
		full: ['mcp:all']
	}

	/** The access a token's MCP scopes give: `mcp:all` and how it is narrowed, or else a
	 *  selection of scripts, flows and endpoints. */
	export function mcpAccessOf(scopes: string[]): McpAccess {
		if (!scopes.includes('mcp:all')) return scopes.length > 0 ? 'selected' : 'full'
		if (scopes.includes('mcp:read_only')) return 'read'
		if (scopes.includes('mcp:read_run')) return 'read_run'
		return 'full'
	}
</script>

<script lang="ts">
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import McpScopeSelector from './McpScopeSelector.svelte'

	interface Props {
		/** The workspace the selection picker browses. */
		workspaceId: string
		/** The token's MCP scopes, the workspace list aside. */
		scopes?: string[]
		access?: McpAccess
		initialScopes?: string[]
	}

	let {
		workspaceId,
		scopes = $bindable(),
		access = $bindable(),
		initialScopes = []
	}: Props = $props()

	const initialAccess = mcpAccessOf(initialScopes)
	access = initialAccess
	const initialSelection = initialAccess === 'selected' ? initialScopes.join(' ') : undefined
	let selection = $state(initialSelection ?? 'mcp:favorites')

	$effect(() => {
		scopes =
			access === 'selected' || access === undefined
				? selection.split(' ').filter((s) => s.length > 0)
				: ACCESS_SCOPES[access]
	})
</script>

<div class="flex flex-col gap-4">
	<div>
		<span class="block mb-1 text-emphasis text-xs font-semibold">Access</span>
		<ToggleButtonGroup bind:selected={access} allowEmpty={false}>
			{#snippet children({ item })}
				<ToggleButton
					{item}
					value="read"
					label="Read only"
					tooltip="Explore and read. Nothing is run or changed."
				/>
				<ToggleButton
					{item}
					value="read_run"
					label="Read and run"
					tooltip="Explore, and run deployed scripts and flows. Nothing is changed."
				/>
				<ToggleButton
					{item}
					value="full"
					label="Read, run and build"
					tooltip="Everything you can do yourself: write, test and deploy drafts, run, and explore."
				/>
				<ToggleButton
					{item}
					value="selected"
					label="Run selected scripts and flows"
					tooltip="Only the scripts and flows you pick, each as its own tool, and nothing else. To share a few automations."
				/>
			{/snippet}
		</ToggleButtonGroup>
	</div>
	{#if access === 'selected'}
		<McpScopeSelector
			{workspaceId}
			bind:scope={selection}
			initialScope={initialSelection}
			runnablesOnly
		/>
	{/if}
</div>
