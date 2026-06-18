<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Settings } from 'lucide-svelte'

	interface Props {
		hasSmtpConfig: boolean
		openSmtpSettings?: () => void
	}
	let { hasSmtpConfig, openSmtpSettings }: Props = $props()
</script>

<div class="flex items-center gap-2">
	{#if hasSmtpConfig}
		<Badge color="green">SMTP configured</Badge>
	{:else}
		<Badge color="red">SMTP not configured</Badge>
	{/if}
	{#if openSmtpSettings}
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: Settings }}
			onclick={openSmtpSettings}>Configure SMTP</Button
		>
	{:else}
		<span class="text-xs text-secondary">SMTP is configured by the instance admin</span>
	{/if}
</div>
