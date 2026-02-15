<script lang="ts">
	import { clickOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { ScriptService, FlowService } from '$lib/gen'
	import SelectDropdown from './select/SelectDropdown.svelte'
	import {
		inputBaseClass,
		inputBorderClass,
		inputSizeClasses
	} from './text_input/TextInput.svelte'
	import { ButtonType } from './common/button/model'
	import type { ProcessedItem } from './select/utils.svelte'

	let {
		value = $bindable(''),
		workspace = '',
		kind = '',
		ownerPrefix = '',
		disabled = false,
		autofocus = false,
		placeholder = '',
		error = false,
		id,
		onKeyUp
	}: {
		value?: string
		workspace?: string
		kind?: string
		ownerPrefix?: string
		disabled?: boolean
		autofocus?: boolean
		placeholder?: string
		error?: string | boolean
		id?: string
		onKeyUp?: (e: KeyboardEvent) => void
	} = $props()

	let paths: string[] = $state([])
	let open = $state(false)
	let loaded = $state(false)
	let inputEl: HTMLInputElement | undefined = $state()

	async function loadPaths() {
		if (loaded || !workspace || !['script', 'flow', 'app'].includes(kind)) return
		try {
			const [scriptPaths, flowPaths] = await Promise.all([
				ScriptService.listScriptPaths({ workspace }).catch(() => []),
				FlowService.listFlowPaths({ workspace }).catch(() => [])
			])
			paths = [...scriptPaths, ...flowPaths]
		} catch {
			paths = []
		}
		loaded = true
	}

	function handleFocus() {
		loadPaths()
		open = true
	}

	let currentDir = $derived(value.includes('/') ? value.substring(0, value.lastIndexOf('/') + 1) : '')
	let filterText = $derived(value.substring(currentDir.length))

	let suggestions: ProcessedItem<string>[] = $derived.by(() => {
		if (!paths.length || !open) return []
		const prefix = ownerPrefix + '/' + currentDir
		const segments: Set<string> = new Set()
		for (const path of paths) {
			if (!path.startsWith(prefix)) continue
			const remaining = path.slice(prefix.length)
			if (!remaining) continue
			const slashIdx = remaining.indexOf('/')
			if (slashIdx !== -1) {
				segments.add(remaining.slice(0, slashIdx + 1))
			}
		}
		return Array.from(segments)
			.filter((s) => s.toLowerCase().includes(filterText.toLowerCase()))
			.sort((a, b) => a.localeCompare(b))
			.slice(0, 50)
			.map((s) => ({ label: s, value: s }))
	})

	function selectValue(item: ProcessedItem<string>) {
		value = currentDir + item.value
		if (item.value.endsWith('/')) {
			loadPaths()
		} else {
			open = false
		}
		inputEl?.focus()
	}

	function handleKeyUp(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			open = false
		}
		onKeyUp?.(e)
	}

	export function focus() {
		inputEl?.focus()
	}
</script>

<div class="relative" use:clickOutside={{ onClickOutside: () => (open = false) }}>
	<!-- svelte-ignore a11y_autofocus -->
	<input
		bind:this={inputEl}
		bind:value
		{disabled}
		{autofocus}
		{placeholder}
		{id}
		type="text"
		autocomplete="off"
		class={twMerge(
			inputBaseClass,
			inputSizeClasses['md'],
			ButtonType.UnifiedHeightClasses['md'],
			inputBorderClass({ error: !!error, forceFocus: open }),
			'w-full'
		)}
		onfocus={handleFocus}
		onkeyup={handleKeyUp}
	/>
	{#if suggestions.length > 0}
		<SelectDropdown
			disablePortal
			{open}
			{disabled}
			processedItems={suggestions}
			value={undefined}
			filterText=""
			onSelectValue={selectValue}
			getInputRect={inputEl && (() => inputEl!.getBoundingClientRect())}
			noItemsMsg="No matches"
		/>
	{/if}
</div>
