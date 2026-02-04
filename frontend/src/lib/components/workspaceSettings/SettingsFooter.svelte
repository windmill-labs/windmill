<script lang="ts">
	import { Badge, Button } from '../common'
	import { Save, X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'

	let {
		hasUnsavedChanges = false,
		onSave,
		onDiscard,
		saveLabel = 'Save settings',
		disabled = false
	}: {
		hasUnsavedChanges?: boolean
		onSave: () => void | Promise<void>
		onDiscard: () => void
		saveLabel?: string
		disabled?: boolean
	} = $props()
</script>

<div class="sticky bottom-0 z-10 w-full border-t bg-surface">
	<div class="flex items-center justify-between pt-4 pb-8">
		<div class="flex items-center gap-3">
			{#if hasUnsavedChanges}
				<div transition:fade={{ duration: 150 }}>
					<Badge color="yellow">Unsaved changes</Badge>
				</div>
			{/if}
		</div>

		<div class="flex items-center gap-2">
			{#if hasUnsavedChanges}
				<div transition:fade={{ duration: 150 }}>
					<Button variant="default" size="sm" startIcon={{ icon: X }} on:click={onDiscard}>
						Discard
					</Button>
				</div>
			{/if}

			<Button
				variant="accent"
				unifiedSize="md"
				startIcon={{ icon: Save }}
				disabled={!hasUnsavedChanges || disabled}
				on:click={onSave}
			>
				{saveLabel}
			</Button>
		</div>
	</div>
</div>
