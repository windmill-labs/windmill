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
		/** Read-only flag; also forwarded to McpScopeSelector to filter incompatible
		 *  endpoints/runnables. Two-way bound so the inline toggle below the
		 *  "Limit token permissions" switch (and the MCP variant) writes back. */
		readOnly?: boolean
	}

	let {
		mode,
		workspaceId = '',
		initialScopes,
		value = $bindable(),
		readOnly = $bindable(false)
	}: Props = $props()

	// In standard mode, only meaningful when the user has turned "Limit token
	// permissions" on. Reset when they un-limit so the flag doesn't quietly
	// stick if they re-enable later.
	$effect(() => {
		if (mode === 'standard' && !limited && readOnly) {
			readOnly = false
		}
	})

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
			<ScopeSelector bind:selectedScopes={standardScopes}>
				{#snippet topSlot()}
					<div class="text-tertiary">
						<Toggle
							bind:checked={readOnly}
							options={{
								right: 'Read-only',
								rightTooltip:
									'Restricts this token to GET/HEAD endpoints. Any mutating request (POST/PUT/PATCH/DELETE) or job-run action will be rejected with 403, regardless of the scopes selected below.'
							}}
							size="2xs"
						/>
					</div>
				{/snippet}
			</ScopeSelector>
		{/if}
	</div>
{:else}
	<div class="flex flex-col gap-2">
		<div class="text-tertiary">
			<Toggle
				bind:checked={readOnly}
				options={{
					right: 'Read-only',
					rightTooltip:
						'Restricts this MCP URL to read-only endpoints. The LLM will only see read tools — script and flow runs will be hidden.'
				}}
				size="2xs"
			/>
		</div>
		<McpScopeSelector
			{workspaceId}
			bind:scope={mcpScope}
			initialScope={initialMcpScope}
			{readOnly}
		/>
	</div>
{/if}
