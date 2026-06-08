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
	import McpOAuthConnect from './McpOAuthConnect.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	interface Props {
		tool: McpTool
	}

	type McpResourceValue = {
		name?: string
		url: string
		token?: string | null
	}

	type OAuthDiscoveryStatus = 'none' | 'discovering' | 'supported' | 'unsupported'

	function getMcpResourceValue(value: unknown): McpResourceValue | undefined {
		if (!value || typeof value !== 'object') {
			return undefined
		}

		const candidate = value as Record<string, unknown>
		const url = typeof candidate.url === 'string' ? candidate.url : undefined

		if (!url) {
			return undefined
		}

		return {
			name: typeof candidate.name === 'string' ? candidate.name : undefined,
			url,
			token:
				typeof candidate.token === 'string' || candidate.token === null
					? candidate.token
					: undefined
		}
	}

	function getFallbackMcpResourceName(path: string, url?: string): string {
		if (path) {
			const parts = path.split('/')
			return parts[parts.length - 1] || path
		}

		if (url) {
			try {
				return new URL(url).hostname.replace(/\./g, '_')
			} catch {
				return 'mcp_server'
			}
		}

		return 'mcp_server'
	}

	function getErrorMessage(error: any): string | undefined {
		return error?.body?.message || error?.body || error?.message
	}

	function isUnauthorizedError(error: any): boolean {
		const status = error?.status
		const message = String(getErrorMessage(error) ?? '')

		return (
			status === 401 ||
			status === 403 ||
			/\b(unauthorized|unauthorised|401|403)\b/i.test(message)
		)
	}

	let { tool = $bindable() }: Props = $props()

	let refreshCount = $state(0)
	let resourcePicker: ResourcePicker | undefined = $state()
	let lastResourcePath = $state<string | undefined>()
	let oauthDiscoveryStatus = $state<OAuthDiscoveryStatus>('none')

	let resourcePath = $derived(tool.value.resource_path)

	let tools = usePromise(
		async () =>
			await loadToolsCached({
				workspace: $workspaceStore!,
				path: resourcePath,
				refreshCount
			}),
		{ loadInit: false }
	)

	let selectedResource = usePromise(
		async () =>
			resourcePath && $workspaceStore
				? await ResourceService.getResource({
						workspace: $workspaceStore,
						path: resourcePath
					})
				: undefined,
		{ loadInit: false }
	)

	let toolOptions = $derived(safeSelectItems((tools.value ?? []).map((t) => t.name)))
	let error = $derived(getErrorMessage(tools.error))
	let selectedMcpResourceValue = $derived(getMcpResourceValue(selectedResource.value?.value))
	let selectedMcpHasToken = $derived(
		typeof selectedMcpResourceValue?.token === 'string' &&
			selectedMcpResourceValue.token.trim().length > 0
	)
	let selectedMcpUrlOnly = $derived(Boolean(selectedMcpResourceValue?.url && !selectedMcpHasToken))
	let selectedMcpServerUrl = $derived(selectedMcpResourceValue?.url)
	let selectedMcpResourceName = $derived(
		selectedMcpResourceValue?.name ||
			getFallbackMcpResourceName(resourcePath, selectedMcpServerUrl)
	)
	let selectedMcpUnauthorized = $derived(
		Boolean(selectedMcpUrlOnly && isUnauthorizedError(tools.error))
	)
	let connectionEstablished = $derived(
		Boolean(resourcePath?.length > 0 && tools.status !== 'loading' && !tools.error && tools.value)
	)
	let oauthDiscoveryPending = $derived(
		selectedMcpUnauthorized &&
			(oauthDiscoveryStatus === 'none' || oauthDiscoveryStatus === 'discovering')
	)
	let showConnecting = $derived(
		Boolean(
			resourcePath?.length > 0 &&
				!connectionEstablished &&
				(tools.status === 'loading' || selectedResource.status === 'loading' || oauthDiscoveryPending)
		)
	)
	let showOAuthFlow = $derived(
		selectedMcpUnauthorized && oauthDiscoveryStatus !== 'unsupported'
	)
	let connectionError = $derived(
		resourcePath?.length > 0 &&
			!connectionEstablished &&
			!showConnecting &&
			(!selectedMcpUnauthorized || oauthDiscoveryStatus === 'unsupported')
			? error
			: undefined
	)

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
		resourcePath
		$workspaceStore
		untrack(() => {
			if (resourcePath?.length > 0 && $workspaceStore) {
				selectedResource.refresh()
			} else {
				selectedResource.clear()
			}
		})
	})

	$effect(() => {
		const currentPath = resourcePath
		untrack(() => {
			if (currentPath !== lastResourcePath) {
				lastResourcePath = currentPath
				oauthDiscoveryStatus = 'none'
			}
		})
	})

	$effect(() => {
		if (!selectedMcpUnauthorized && oauthDiscoveryStatus !== 'none') {
			oauthDiscoveryStatus = 'none'
		}
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

	function handleOAuthDiscoveryStatus(status: Exclude<OAuthDiscoveryStatus, 'none'>) {
		oauthDiscoveryStatus = status
	}

	async function handleOAuthConnected(connectedResourcePath: string, resourceName: string) {
		await resourcePicker?.refreshResources()
		tool.value.resource_path = connectedResourcePath
		tool.summary = `MCP: ${resourceName}`
		oauthDiscoveryStatus = 'none'
		selectedResource.refresh()
		refreshCount += 1
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
			<ResourcePicker bind:this={resourcePicker} resourceType="mcp" bind:value={tool.value.resource_path} />
		</Label>
	</div>

	{#if showConnecting}
		<div class="text-xs text-secondary italic">connecting to mcp server...</div>
	{/if}

	{#if showOAuthFlow}
		{#key `${resourcePath ?? ''}:${selectedMcpServerUrl ?? ''}`}
			<McpOAuthConnect
				onConnected={handleOAuthConnected}
				onDiscoveryStatus={handleOAuthDiscoveryStatus}
				initialServerUrl={selectedMcpServerUrl}
				initialResourceName={selectedMcpResourceName}
				initialResourcePath={resourcePath}
				updateExistingResource={true}
				hideUntilDiscovered={true}
			/>
		{/key}
	{/if}

	{#if connectionError}
		<div class="text-xs text-red-600 dark:text-red-400">{`Failed to connect to MCP server: ${connectionError}`}</div>
	{/if}

	{#if connectionEstablished}
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
				<div class="max-h-48 overflow-y-auto border rounded p-2 bg-surface-secondary">
					{#if (tools.value ?? []).length === 0}
						<div class="text-xs text-secondary italic">No tools returned by this MCP server.</div>
					{:else}
						<div class="flex flex-col gap-1">
							{#each tools.value ?? [] as mcpTool (mcpTool.name)}
								<div class="text-xs">
									<span class="font-semibold">{mcpTool.name}</span>
									{#if mcpTool.description}
										<span class="text-secondary">— {mcpTool.description}</span>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
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
