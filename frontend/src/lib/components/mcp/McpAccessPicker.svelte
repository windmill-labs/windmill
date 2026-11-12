<script lang="ts" module>
	export type McpAccess = 'read' | 'read_run' | 'full' | 'selected'

	/** Marks a selection, so the per-workspace URLs add no API endpoint tools to it. */
	export const RUNNABLES_AS_TOOLS = 'mcp:runnables_as_tools'

	/** Carried by every build-and-run `mcp:all` token Windmill makes, so the tools server's URLs
	 *  serve it the AI session's tools; a plain `mcp:all` is an older token and keeps the endpoint
	 *  tools. Grants nothing, so it is never shown. */
	export const MCP_SESSION = 'mcp:session'

	const ACCESS_SCOPES: Record<Exclude<McpAccess, 'selected'>, string[]> = {
		read: ['mcp:all', 'mcp:read_only'],
		read_run: ['mcp:all', 'mcp:read_run'],
		full: ['mcp:all', MCP_SESSION]
	}

	/** The access a token's MCP scopes give: `mcp:all` and how it is narrowed, or else a
	 *  selection of scripts, flows and endpoints. */
	export function mcpAccessOf(scopes: string[]): McpAccess {
		scopes = scopes.filter((s) => s !== RUNNABLES_AS_TOOLS && s !== MCP_SESSION)
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
	import Popover from '$lib/components/Popover.svelte'
	import { Badge } from '$lib/components/common'
	import Section from '$lib/components/Section.svelte'
	import { SvelteMap } from 'svelte/reactivity'
	import type { Snippet } from 'svelte'
	import { resource } from 'runed'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { resolveSessionAccess } from '$lib/components/copilot/chat/global/sessionAccess'
	import { accessNarrowing } from './accessNarrowing'
	import { userWorkspaces } from '$lib/stores'

	interface Props {
		/** The workspace the selection picker browses. */
		workspaceId: string
		/** The workspaces the token reaches, to warn where one holds it to less than `access`. */
		workspaces?: string[]
		/** The token's MCP scopes, the workspace list aside. */
		scopes?: string[]
		access?: McpAccess
		initialScopes?: string[]
		/** Rendered between the access choice and the tool list. */
		afterAccess?: Snippet
	}

	let {
		workspaceId,
		workspaces = [],
		scopes = $bindable(),
		access = $bindable(),
		initialScopes = [],
		afterAccess
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

	// Each workspace's access, as the chat resolves it, to compare with the level picked.
	const workspaceAccess = resource(
		// Rerun once the workspace list loads: until then a role reads as its legacy defaults.
		() => ({ ids: [...workspaces], roles: $userWorkspaces }),
		({ ids }) => Promise.all(ids.map(async (id) => ({ id, there: await resolveSessionAccess(id) })))
	)
	const narrowed = $derived(
		access
			? (workspaceAccess.current ?? []).flatMap(({ id, there }) => {
					const why = accessNarrowing(access!, there)
					return why ? [{ id, why }] : []
				})
			: []
	)
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
					tooltip="Explore, and run deployed scripts, flows and agents. The client changes nothing itself; what it runs can."
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
	{#if narrowed.length > 0}
		<Alert type="warning" size="xs" title="Narrower in some workspaces">
			<ul class="flex flex-col gap-0.5">
				{#each narrowed as { id, why } (id)}
					<li>In <span class="font-semibold">{id}</span>, {why}.</li>
				{/each}
			</ul>
		</Alert>
	{/if}
	{@render afterAccess?.()}
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
		<!-- A selection names paths of one workspace, so switching workspace starts it over. -->
		{#key workspaceId}
			<McpScopeSelector
				{workspaceId}
				bind:scope={selection}
				initialScope={initialSelection}
				runnablesOnly
			/>
		{/key}
	{/if}
</div>
