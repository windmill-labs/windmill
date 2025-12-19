<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Plug, RotateCw, Unplug } from 'lucide-svelte'
	import { base } from '$lib/base'

	interface Props {
		isConnected: boolean | undefined
		slackTeamName?: string
		mode: 'instance' | 'workspace'
		onRefresh?: () => void
		onDisconnect?: () => void
	}

	let { isConnected, slackTeamName, mode, onRefresh, onDisconnect }: Props = $props()

	// Connection URLs based on mode
	let connectUrl = $derived(
		mode === 'instance'
			? `${base}/api/oauth/connect_slack?instance=true`
			: `${base}/workspace_settings?tab=slack`
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
		<div class="flex items-center gap-2">
			<Badge color="green">
				<Plug size={14} />
				<span class="text-xs text-primary">
					{#if slackTeamName}
						{mode === 'instance' ? 'Instance' : 'Workspace'} connected to the Slack workspace '{slackTeamName}'
					{:else}
						{mode === 'instance' ? 'Instance' : 'Workspace'} connected to Slack workspace
					{/if}
				</span>
			</Badge>
		</div>
		<!-- Disconnect Button -->
		{#if onDisconnect}
			<Button
				variant="default"
				unifiedSize="sm"
				onclick={onDisconnect}
				startIcon={{ icon: Unplug }}
				destructive
			>
				Disconnect slack
			</Button>
		{/if}
	{:else}
		<!-- Not Connected - Show connect button -->

		<Badge color="red">
			<Unplug size={14} />
			<span class="text-xs text-primary"
				>{mode === 'instance' ? 'Instance' : 'Workspace'} not connected to Slack</span
			>
		</Badge>

		<a href={connectUrl} class="text-xs"
			>{mode === 'instance' ? 'Connect instance to Slack' : 'Open workspace slack settings'}</a
		>
	{/if}
</div>
