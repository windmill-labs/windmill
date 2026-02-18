<script lang="ts">
	import { Button } from './common'
	import { type ButtonType } from './common/button/model'
	import { Save, CheckCircle2, AlertCircle, Loader2 } from 'lucide-svelte'
	import { fly } from 'svelte/transition'

	let {
		onSave,
		disabled = false,
		label = 'Save settings',
		size = undefined,
		unifiedSize = undefined,
		variant = 'accent'
	}: {
		onSave: () => void | Promise<void>
		disabled?: boolean
		label?: string
		size?: ButtonType.Size | undefined
		unifiedSize?: ButtonType.UnifiedSize | undefined
		variant?: ButtonType.Variant
	} = $props()

	let isSaving = $state(false)
	let saveStatus: 'success' | 'error' | null = $state(null)
	let statusTimeout: ReturnType<typeof setTimeout> | null = null

	function clearStatusTimeout() {
		if (statusTimeout !== null) {
			clearTimeout(statusTimeout)
			statusTimeout = null
		}
	}

	function handleClick() {
		if (isSaving) return

		isSaving = true
		saveStatus = null
		clearStatusTimeout()

		Promise.resolve(onSave())
			.then(() => {
				saveStatus = 'success'
				statusTimeout = setTimeout(() => {
					saveStatus = null
					statusTimeout = null
				}, 1500)
			})
			.catch((error) => {
				console.error('Save failed:', error)
				saveStatus = 'error'
				statusTimeout = setTimeout(() => {
					saveStatus = null
					statusTimeout = null
				}, 3000)
			})
			.finally(() => {
				isSaving = false
			})
	}

	$effect(() => {
		return () => {
			clearStatusTimeout()
		}
	})
</script>

<div class="relative overflow-hidden rounded-md">
	<Button
		{variant}
		{size}
		{unifiedSize}
		startIcon={{
			icon: isSaving ? Loader2 : Save,
			classes: isSaving ? 'animate-spin' : ''
		}}
		disabled={disabled || isSaving}
		onClick={handleClick}
	>
		{isSaving ? 'Saving...' : label}
	</Button>

	{#if saveStatus === 'success'}
		<div
			class="absolute inset-0 flex items-center justify-center bg-green-200 dark:bg-green-800 rounded-md"
			transition:fly={{ y: -10, duration: 300 }}
		>
			<CheckCircle2 class="w-5 h-5 text-green-700 dark:text-green-300" />
		</div>
	{:else if saveStatus === 'error'}
		<div
			class="absolute inset-0 flex items-center justify-center bg-red-500 dark:bg-red-700 rounded-md"
			transition:fly={{ y: -10, duration: 300 }}
		>
			<AlertCircle class="w-5 h-5 text-white" />
		</div>
	{/if}
</div>
