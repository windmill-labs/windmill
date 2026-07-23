<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Loader2, Settings } from 'lucide-svelte'

	interface Props {
		// Human label of the setting, e.g. "Concurrency limit" or "Cache".
		label: string
		// Whether the setting is currently configured on the referenced script.
		active: boolean
		// Rendered summary of the current value (only shown when active).
		valueText?: string | undefined
		loading?: boolean
		// Editing is possible only for non-hub, non-hash-pinned workspace scripts.
		canEdit: boolean
		onEdit?: () => void
	}

	let { label, active, valueText, loading = false, canEdit, onEdit }: Props = $props()
</script>

<div class="flex flex-col gap-2 rounded-md border p-3 bg-surface-secondary">
	<div class="flex flex-row items-center justify-between gap-2">
		<span class="text-xs text-secondary">
			{label} is configured on the referenced workspace script.
		</span>
		{#if canEdit}
			<Button size="xs" variant="border" startIcon={{ icon: Settings }} on:click={() => onEdit?.()}>
				Edit script settings
			</Button>
		{/if}
	</div>
	<div class="text-xs">
		{#if loading}
			<span class="text-tertiary inline-flex items-center gap-1">
				<Loader2 class="animate-spin" size={12} /> Loading current value…
			</span>
		{:else if active}
			<span class="font-semibold text-emphasis">{valueText}</span>
		{:else}
			<span class="text-tertiary">Not set on the script.</span>
		{/if}
	</div>
	{#if !canEdit}
		<span class="text-2xs text-tertiary">
			Hub scripts and steps pinned to a specific version cannot be edited from here.
		</span>
	{/if}
</div>
