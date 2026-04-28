<script lang="ts">
	import Toggle from '../Toggle.svelte'
	import ScopeSelector from './ScopeSelector.svelte'
	import McpScopeSelector from '../mcp/McpScopeSelector.svelte'

	interface Props {
		mode: 'standard' | 'mcp'
		workspaceId?: string
		/** Existing scopes (used for both initial standard selection and parsing MCP scope) */
		initialScopes?: string[]
		/** Final scope value: null = unrestricted/full access, array = explicit list */
		value: string[] | null
	}

	let { mode, workspaceId = '', initialScopes, value = $bindable() }: Props = $props()

	const initialMcpScope = $derived(
		(initialScopes ?? []).length > 0 ? (initialScopes ?? []).join(' ') : undefined
	)

	// Standard-mode local state, seeded from initialScopes if any.
	let limited = $state((initialScopes ?? []).length > 0)
	let standardScopes = $state<string[]>([...(initialScopes ?? [])])

	// MCP-mode local state. When no initial scope is provided we fall back
	// to `mcp:favorites` (matches the create-token default).
	let mcpScope = $state(initialMcpScope ?? 'mcp:favorites')

	$effect(() => {
		if (mode === 'mcp') {
			const parts = mcpScope
				.split(' ')
				.map((s) => s.trim())
				.filter((s) => s.length > 0)
			value = parts.length > 0 ? parts : null
		} else {
			value = limited && standardScopes.length > 0 ? standardScopes : null
		}
	})
</script>

{#if mode === 'standard'}
	<div class="flex flex-col gap-2">
		<Toggle
			bind:checked={limited}
			options={{
				right: 'Limit token permissions',
				rightTooltip:
					'When off, the token has full API access. Turn on to restrict it to specific scopes.'
			}}
			size="xs"
		/>
		{#if limited}
			<ScopeSelector bind:selectedScopes={standardScopes} />
		{/if}
	</div>
{:else}
	<McpScopeSelector {workspaceId} bind:scope={mcpScope} initialScope={initialMcpScope} />
{/if}
