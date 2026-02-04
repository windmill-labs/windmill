<script lang="ts">
	import { Button } from '../common'
	import { Save, X, CheckCircle2, AlertCircle, Loader2 } from 'lucide-svelte'
	import { fade, fly } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'

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

	let saveStatus: 'success' | 'error' | null = $state(null)
	let isSaving = $state(false)
	let statusTimeout: ReturnType<typeof setTimeout> | null = null

	function clearStatusTimeout() {
		if (statusTimeout !== null) {
			clearTimeout(statusTimeout)
			statusTimeout = null
		}
	}

	async function handleSave() {
		if (isSaving) return

		isSaving = true
		saveStatus = null
		clearStatusTimeout() // Clear any existing timeout

		try {
			await onSave()
			saveStatus = 'success'
			// Auto-hide success status after 3 seconds
			statusTimeout = setTimeout(() => {
				saveStatus = null
				statusTimeout = null
			}, 3000)
		} catch (error) {
			console.error('Save failed:', error)
			saveStatus = 'error'
			// Auto-hide error status after 5 seconds (longer for errors)
			statusTimeout = setTimeout(() => {
				saveStatus = null
				statusTimeout = null
			}, 5000)
		} finally {
			isSaving = false
		}
	}

	// Cleanup timeout on component unmount
	$effect(() => {
		return () => {
			clearStatusTimeout()
		}
	})
</script>

<div class={twMerge('sticky bottom-0 z-10 w-full border-t bg-surface-tertiary')}>
	<div class="flex items-center justify-end pt-4 pb-8">
		<div class="flex items-center gap-2">
			{#if hasUnsavedChanges}
				<div transition:fade={{ duration: 150 }}>
					<Button
						variant="default"
						size="sm"
						startIcon={{ icon: X }}
						on:click={onDiscard}
						disabled={isSaving}
					>
						Discard
					</Button>
				</div>
			{/if}

			<div class="relative">
				<Button
					variant="accent"
					unifiedSize="md"
					startIcon={{
						icon: isSaving ? Loader2 : Save,
						classes: isSaving ? 'animate-spin' : ''
					}}
					disabled={!hasUnsavedChanges || disabled || isSaving}
					on:click={handleSave}
				>
					{isSaving ? 'Saving...' : saveLabel}
				</Button>

				<!-- Success icon overlay -->
				{#if saveStatus === 'success'}
					<div
						class="absolute inset-0 flex items-center justify-center bg-green-200 rounded-md"
						transition:fly={{ y: -10, duration: 300 }}
					>
						<CheckCircle2 class="w-5 h-5 text-green-700" />
					</div>
				{:else if saveStatus === 'error'}
					<div
						class="absolute inset-0 flex items-center justify-center bg-red-500 rounded-md"
						transition:fly={{ y: -10, duration: 300 }}
					>
						<AlertCircle class="w-5 h-5 text-white" />
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>
