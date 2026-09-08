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
		// Set when loading the referenced script failed, to distinguish that from "not set".
		error?: string | undefined
		// Editing is possible only for non-hub, non-hash-pinned workspace scripts.
		canEdit: boolean
		// Explanation shown when editing isn't possible (varies: hub, pinned, dev editor).
		noEditReason?: string | undefined
		onEdit?: () => void
	}

	let {
		label,
		active,
		valueText,
		loading = false,
		error = undefined,
		canEdit,
		noEditReason = undefined,
		onEdit
	}: Props = $props()
</script>

<div class="flex flex-col gap-2 rounded-md border p-3 bg-surface-secondary">
	<div class="flex flex-row items-center justify-between gap-2">
		<span class="text-xs text-secondary">
			{label} is managed on the referenced workspace script.
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
		{:else if error}
			<span class="text-red-500">Could not load the current value: {error}</span>
		{:else if active}
			<span class="font-semibold text-emphasis">{valueText}</span>
		{:else}
			<span class="text-tertiary">Not set on the script.</span>
		{/if}
	</div>
	{#if !canEdit && noEditReason}
		<span class="text-2xs text-tertiary">{noEditReason}</span>
	{/if}
</div>
