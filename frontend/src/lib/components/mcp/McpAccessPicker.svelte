<script lang="ts" module>
	export type McpAccess = 'read' | 'read_run' | 'full' | 'selected'

	/** Marks a selection, so the per-workspace URLs add no API endpoint tools to it. */
	export const RUNNABLES_AS_TOOLS = 'mcp:runnables_as_tools'

	/** Added by the server to an `mcp:all` sign-in so the tools server's URLs serve it the AI
	 *  session's tools. Grants nothing, so it is never shown. */
	export const MCP_SESSION = 'mcp:session'

	const ACCESS_SCOPES: Record<Exclude<McpAccess, 'selected'>, string[]> = {
		read: ['mcp:all', 'mcp:read_only'],
		read_run: ['mcp:all', 'mcp:read_run'],
		full: ['mcp:all']
	}

	/** The access a token's MCP scopes give: `mcp:all` and how it is narrowed, or else a
	 *  selection of scripts, flows and endpoints. */
	export function mcpAccessOf(scopes: string[]): McpAccess {
		scopes = scopes.filter((s) => s !== RUNNABLES_AS_TOOLS && s !== MCP_SESSION)
		if (!scopes.includes('mcp:all')) return scopes.length > 0 ? 'selected' : 'full'
		// Both narrowings at once is a client requesting every advertised scope, not a choice.
		if (scopes.includes('mcp:read_only') && scopes.includes('mcp:read_run')) return 'full'
		if (scopes.includes('mcp:read_only')) return 'read'
		if (scopes.includes('mcp:read_run')) return 'read_run'
		return 'full'
	}
</script>

<script lang="ts">
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import McpScopeSelector from './McpScopeSelector.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Badge } from '$lib/components/common'
	import Section from '$lib/components/Section.svelte'
	import { SvelteMap } from 'svelte/reactivity'

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
	const initialSelection =
		initialAccess === 'selected'
			? initialScopes.filter((s) => s !== RUNNABLES_AS_TOOLS && s !== MCP_SESSION).join(' ')
			: undefined
	let selection = $state(initialSelection ?? 'mcp:favorites')

	$effect(() => {
		scopes =
			access === 'selected' || access === undefined
				? [
						...selection.split(' ').filter((s) => s.length > 0 && s !== RUNNABLES_AS_TOOLS),
						RUNNABLES_AS_TOOLS
					]
				: ACCESS_SCOPES[access]
	})

	// The tools each access level gives, as the server lists them.
	type ToolsetEntry = { name: string; description: string }
	let toolsCollapsed = $state(true)
	const toolsets = new SvelteMap<string, ToolsetEntry[]>()
	const toolsetAccess = $derived(access === 'selected' ? undefined : access)
	$effect(() => {
		const level = toolsetAccess
		if (toolsCollapsed || !level || toolsets.has(level)) return
		fetch(`/api/mcp/toolset?access=${level}`)
			.then((r) => (r.ok ? r.json() : []))
			.then((tools: ToolsetEntry[]) => toolsets.set(level, tools))
			.catch(() => toolsets.set(level, []))
	})
	const tools = $derived(toolsetAccess ? toolsets.get(toolsetAccess) : undefined)
</script>

<div class="flex flex-col gap-4">
	<div>
		<span class="block mb-1 text-emphasis text-xs font-semibold">Access</span>
		<ToggleButtonGroup bind:selected={access} allowEmpty={false}>
			{#snippet children({ item })}
				<ToggleButton
					{item}
					value="full"
					label="Build and run"
					tooltip="Everything you can do yourself: explore, write, test and deploy drafts, and run."
				/>
				<ToggleButton
					{item}
					value="read_run"
					label="Read and run"
					tooltip="Explore, and run deployed scripts and flows. The client changes nothing itself; the scripts and flows it runs can."
				/>
				<ToggleButton
					{item}
					value="read"
					label="Read only"
					tooltip="Explore and read. Nothing is run or changed."
				/>
				<ToggleButton
					{item}
					value="selected"
					label="Scripts and flows as tools"
					tooltip="The scripts and flows you pick, each as its own tool, and nothing else. To share a few automations."
				/>
			{/snippet}
		</ToggleButtonGroup>
	</div>
	{#if toolsetAccess}
		<Section label="Tools the client gets" small collapsable bind:collapsed={toolsCollapsed}>
			<div class="flex flex-col gap-1 -mt-2">
				<span class="text-2xs text-secondary">
					{tools ? `${tools.length} tools. Hover one for what it does.` : 'Loading tools…'}
				</span>
				<div class="flex flex-wrap gap-1">
					{#each tools ?? [] as tool (tool.name)}
						<Popover notClickable>
							{#snippet text()}
								<div class="text-xs max-w-md">{tool.description}</div>
							{/snippet}
							<Badge rounded small color="gray">{tool.name}</Badge>
						</Popover>
					{/each}
				</div>
			</div>
		</Section>
	{/if}
	{#if access === 'selected'}
		<p class="text-2xs text-secondary -mt-2">
			The client can run these scripts and flows and nothing else: it cannot explore the workspace,
			read runs, build, or run anything outside the selection.
		</p>
		<McpScopeSelector
			{workspaceId}
			bind:scope={selection}
			initialScope={initialSelection}
			runnablesOnly
		/>
	{/if}
</div>
