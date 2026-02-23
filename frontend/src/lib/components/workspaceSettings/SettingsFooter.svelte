<script lang="ts">
	import { Button } from '../common'
	import SaveButton from '../SaveButton.svelte'
	import { X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'

	let {
		hasUnsavedChanges = false,
		onSave,
		onDiscard,
		saveLabel = 'Save settings',
		disabled = false,
		inline = false,
		class: className
	}: {
		hasUnsavedChanges?: boolean
		onSave: () => void | Promise<void>
		onDiscard: () => void
		saveLabel?: string
		disabled?: boolean
		inline?: boolean
		class?: string
	} = $props()
</script>

<div
	class={twMerge(inline ? 'w-full' : 'sticky bottom-0 z-10 w-full border-t bg-surface', className)}
>
	<div class={inline ? 'flex items-center justify-end' : 'flex items-center justify-end pt-4 pb-8'}>
		<div class="flex items-center gap-2">
			{#if hasUnsavedChanges}
				<div transition:fade={{ duration: 150 }}>
					<Button
						variant="default"
						size="sm"
						startIcon={{ icon: X }}
						onClick={onDiscard}
					>
						Discard changes
					</Button>
				</div>
			{/if}

			<SaveButton
				{onSave}
				disabled={!hasUnsavedChanges || disabled}
				label={saveLabel}
				unifiedSize="md"
			/>
		</div>
	</div>
</div>
