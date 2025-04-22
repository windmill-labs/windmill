<script lang="ts">
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { workspaceStore } from '$lib/stores'
	import type { TruncatedToken, NewToken } from '$lib/gen'
	import { UserService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { copyToClipboard } from '$lib/utils'
	import { Clipboard, Plus } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	// --- Props ---
	interface Props {
		tokens?: TruncatedToken[]
		onDeleteToken: (tokenPrefix: string) => void
		onListTokens: () => void
	}
	let { tokens = [], onDeleteToken, onListTokens }: Props = $props()

	// --- Local State ---
	let displayCreateMcpUrl = $state(false)
	let newMcpScope = $state('favorites')
	let newMcpUrl = $state<string | undefined>(undefined)

	$effect(() => {
		console.log('displayCreateMcpUrl', displayCreateMcpUrl)
	})
	// --- Functions ---
	async function createMcpUrl(): Promise<void> {
		const scope = `mcp:${newMcpScope}`
		try {
			const newToken = await UserService.createToken({
				requestBody: {
					label: 'MCP token',
					expiration: undefined,
					scopes: [scope],
					workspace_id: $workspaceStore
				} as NewToken
			})
			newMcpUrl = `${window.location.origin}/api/w/${$workspaceStore}/mcp/sse?token=${newToken}`
			displayCreateMcpUrl = false
			onListTokens()
		} catch (err) {
			console.error('Failed to create MCP URL:', err)
		}
	}

	function handleGenerateClick() {
		displayCreateMcpUrl = true
		newMcpUrl = undefined
	}

	function handleCancelClick() {
		displayCreateMcpUrl = false
	}

	function handleCopyClick() {
		copyToClipboard(newMcpUrl ?? '')
	}

	function handleDeleteClick(tokenPrefix: string) {
		onDeleteToken(tokenPrefix)
	}
</script>

<!-- MCP URLs Section -->
<div class="grid grid-cols-2 pt-8 pb-1">
	<h2 class="py-0 my-0 border-b pt-3">MCP URLs</h2>
	<div class="flex justify-end border-b pb-1">
		<Button
			size="sm"
			startIcon={{ icon: Plus }}
			btnClasses={displayCreateMcpUrl ? 'hidden' : ''}
			on:click={handleGenerateClick}
		>
			Generate new URL
		</Button>
	</div>
</div>
<div class="text-2xs text-tertiary italic pb-6">
	URLs for Model Context Protocol (MCP) SSE endpoint. Use these to call your tools and workflows
	from your LLM client.
</div>
<div>
	<div
		class={`border rounded-md mb-6 px-2 py-2 bg-green-50 dark:bg-green-200 dark:text-green-800 flex flex-row flex-wrap ${
			newMcpUrl ? '' : 'hidden'
		}`}
	>
		{#if newMcpUrl}
			<div>
				Generated URL: <button onclick={handleCopyClick} class="inline-flex gap-2 items-center">
					{newMcpUrl}
					<Clipboard size={12} />
				</button>
			</div>
			<div class="pt-1 text-xs ml-2">
				Make sure to copy your new MCP URL now. You won't be able to see the full token again!
			</div>
		{/if}
	</div>
</div>

<!-- MCP URL Creation Interface -->
{#if displayCreateMcpUrl}
	<div class="py-3 px-3 border rounded-md mb-6 bg-surface-secondary min-w-min">
		<h3 class="pb-3 font-semibold">Generate a new MCP URL</h3>
		<div class="flex flex-col gap-3">
			<div>
				<span class="block text-sm font-medium text-secondary mb-1"
					>Scope: Select resources accessible via this URL. 'Favorites Only' restricts access to
					starred scripts/flows. 'All Resources' allows access to any script/flow the owner can run.</span
				>
				<ToggleButtonGroup bind:selected={newMcpScope} allowEmpty={false} let:item>
					<ToggleButton {item} value="favorites" label="Favorites Only" />
					<ToggleButton {item} value="all" label="All Resources" />
				</ToggleButtonGroup>
			</div>
			<div class="flex items-end justify-start">
				<Button on:click={createMcpUrl}>Generate URL</Button>
				<Button btnClasses="ml-2" on:click={handleCancelClick}>Cancel</Button>
			</div>
		</div>
	</div>
{/if}

{#if tokens && tokens.length > 0}
	<div class="overflow-auto mb-8">
		<TableCustom>
			<tr slot="header-row">
				<th>URL (prefix)</th>
				<th>Scope</th>
				<th></th>
			</tr>
			<tbody slot="body">
				{#each tokens as token}
					{@const urlParts = `${window.location.origin}/api/w/${$workspaceStore}/mcp/sse?token=${token.token_prefix}`}
					<tr>
						<td class="grow font-mono text-2xs">{urlParts}</td>
						<td class="grow font-mono text-xs">{token.scopes?.join(', ') ?? ''}</td>
						<td class="grow">
							<button
								class="text-red-500 text-xs underline"
								onclick={() => handleDeleteClick(token.token_prefix)}
							>
								Delete
							</button>
						</td>
					</tr>
				{/each}
			</tbody>
		</TableCustom>
	</div>
{/if}
