<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Plug, RotateCw, Unplug } from 'lucide-svelte'
	import { base } from '$lib/base'

	interface Props {
		isConnected: boolean | undefined
		teamsTeamName?: string
		mode: 'instance' | 'workspace'
		onRefresh?: () => void
		onDisconnect?: () => void
	}

	let { isConnected, teamsTeamName, mode, onRefresh, onDisconnect }: Props = $props()

	// Connection URLs based on mode
	let connectUrl = $derived(
		mode === 'instance' ? `${base}/#superadmin-settings` : `${base}/workspace_settings?tab=teams`
	)
</script>

<div class="flex items-center gap-2">
	{#if isConnected === undefined}
		<!-- Loading State -->
		<RotateCw size={14} class="animate-spin text-secondary" />
		{#if onRefresh}
			<Button
				variant="default"
				unifiedSize="sm"
				onclick={onRefresh}
				startIcon={{ icon: RotateCw }}
			/>
		{/if}
	{:else if isConnected}
		<!-- Connected State - Show status indicator -->
		<Badge color="green">
			<Plug size={14} />
			<span class="text-xs text-primary">
				{#if teamsTeamName}
					Connected to the Teams workspace '{teamsTeamName}'
				{:else}
					Connected to Teams workspace
				{/if}
			</span>
		</Badge>

		<!-- Disconnect Button -->
		{#if onDisconnect}
			<Button
				variant="default"
				unifiedSize="sm"
				onclick={onDisconnect}
				startIcon={{ icon: Unplug }}
				destructive
			>
				Disconnect teams
			</Button>
		{/if}
	{:else}
		<Badge color="red">
			<Unplug size={14} />
			<span class="text-xs text-primary"
				>{mode === 'instance' ? 'Instance' : 'Workspace'} not connected to Teams</span
			>
		</Badge>

		{#if mode === 'instance'}
			<span class="text-xs text-secondary">
				Configure Teams OAuth connection in instance settings
			</span>
		{:else}
			<a href={connectUrl} class="text-xs">open workspace teams settings</a>
		{/if}
	{/if}
</div>
