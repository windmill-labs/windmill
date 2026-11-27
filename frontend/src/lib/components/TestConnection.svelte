<script lang="ts">
	import { Database, Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import {
		canTestConnection,
		connectionTestTooltip,
		testResourceConnection
	} from './resourceConnectionTest'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		workspaceOverride?: string | undefined
		resourceType: string | undefined
		args?: Record<string, any> | any
		buttonTextOverride?: string | undefined
		/** See `testResourceConnection`. */
		viaWorker?: boolean
	}

	let {
		workspaceOverride = undefined,
		resourceType,
		args = {},
		buttonTextOverride = undefined,
		viaWorker = false
	}: Props = $props()

	let loading = $state(false)
	const tooltip = $derived(resourceType ? connectionTestTooltip(resourceType) : undefined)

	async function testConnection() {
		if (!resourceType) return
		loading = true
		try {
			const result = await testResourceConnection({
				workspace: workspaceOverride ?? $operatingWorkspace!,
				resourceType,
				args,
				viaWorker
			})
			sendUserToast(
				result.ok ? 'Connection successful' : 'Connection error: ' + result.error,
				!result.ok
			)
		} finally {
			loading = false
		}
	}
</script>

{#if canTestConnection(resourceType)}
	<div class="flex items-center gap-1">
		<Button spacingSize="sm" size="xs" unifiedSize="md" variant="default" on:click={testConnection}>
			{#if loading}
				<Loader2 class="animate-spin mr-2 !h-4 !w-4" />
			{:else}
				<Database class="mr-2 !h-4 !w-4" />
			{/if}
			{buttonTextOverride ?? 'Test connection'}
		</Button>
		{#if tooltip}
			<Tooltip>
				{#snippet text()}{tooltip}{/snippet}
			</Tooltip>
		{/if}
	</div>
{/if}
