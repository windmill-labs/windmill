<script lang="ts">
	import Badge from './common/badge/Badge.svelte'
	import { Plus, Tag, X } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		labels: string[] | undefined
		onchange?: () => void
		class?: string
	}

	let { labels = $bindable(), onchange, class: clazz = '' }: Props = $props()

	let adding = $state(false)
	let inputValue = $state('')
	let inputEl: HTMLInputElement | undefined = $state()
	let existingLabels: string[] = $state([])
	let selectedIdx = $state(-1)

	let suggestions = $derived(
		existingLabels
			.filter(
				(l) =>
					(!inputValue || l.toLowerCase().includes(inputValue.toLowerCase())) &&
					!(labels ?? []).includes(l)
			)
			.slice(0, 8)
	)
	let trimmedInput = $derived(inputValue.trim())
	let showCreateNew = $derived(
		trimmedInput.length > 0 &&
			!suggestions.some((s) => s.toLowerCase() === trimmedInput.toLowerCase()) &&
			!(labels ?? []).includes(trimmedInput)
	)

	async function loadExistingLabels() {
		try {
			const resp = await fetch(`/api/w/${$workspaceStore}/labels/list`)
			if (resp.ok) existingLabels = await resp.json()
		} catch {}
	}

	function startAdding() {
		adding = true
		inputValue = ''
		selectedIdx = -1
		loadExistingLabels()
		setTimeout(() => inputEl?.focus(), 0)
	}

	function addLabel(value?: string) {
		const v = (value ?? inputValue).trim().slice(0, 50)
		if (!v) {
			adding = false
			return
		}
		if (!labels) {
			labels = []
		}
		if (!labels.includes(v)) {
			labels = [...labels, v]
			onchange?.()
		}
		inputValue = ''
		adding = false
	}

	function removeLabel(label: string) {
		if (labels) {
			labels = labels.filter((l) => l !== label)
			onchange?.()
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault()
			if (selectedIdx >= 0 && selectedIdx < suggestions.length) {
				addLabel(suggestions[selectedIdx])
			} else {
				addLabel() // either "Create new" selected or free text
			}
		} else if (e.key === 'Escape') {
			adding = false
		} else if (e.key === 'ArrowDown') {
			e.preventDefault()
			const maxIdx = suggestions.length + (showCreateNew ? 1 : 0) - 1
			selectedIdx = Math.min(selectedIdx + 1, maxIdx)
		} else if (e.key === 'ArrowUp') {
			e.preventDefault()
			selectedIdx = Math.max(selectedIdx - 1, -1)
		}
	}

	function onBlur() {
		// Delay to allow click on suggestion
		setTimeout(() => {
			if (adding) addLabel()
		}, 150)
	}
</script>

<div class="inline-flex items-center gap-1 ml-0.5 h-5 {clazz}">
	{#each labels ?? [] as label (label)}
		<Badge color="blue" small>
			{label}
			<button class="ml-0.5 hover:text-red-500" onclick={() => removeLabel(label)}>
				<X size={10} />
			</button>
		</Badge>
	{/each}
	{#if adding}
		<div class="relative">
			<input
				bind:this={inputEl}
				bind:value={inputValue}
				onkeydown={onKeydown}
				onblur={onBlur}
				class="text-2xs border border-blue-300 rounded px-1.5 py-0 h-5 max-w-32 outline-none focus:ring-1 focus:ring-blue-400"
				placeholder="label"
			/>
			{#if suggestions.length > 0 || showCreateNew}
				<div
					class="absolute top-6 left-0 z-50 bg-surface border border-light rounded shadow-md max-h-32 overflow-y-auto min-w-32"
				>
					{#each suggestions as suggestion, i}
						<button
							class="w-full text-left text-2xs px-2 py-1 hover:bg-surface-hover {i === selectedIdx
								? 'bg-surface-hover'
								: ''}"
							onmousedown={(e) => {
								e.preventDefault()
								addLabel(suggestion)
							}}
						>
							{suggestion}
						</button>
					{/each}
					{#if showCreateNew}
						<button
							class="w-full text-left text-2xs px-2 py-1 hover:bg-surface-hover text-blue-600 {selectedIdx ===
							suggestions.length
								? 'bg-surface-hover'
								: ''}"
							onmousedown={(e) => {
								e.preventDefault()
								addLabel()
							}}
						>
							+ Create "{trimmedInput}"
						</button>
					{/if}
				</div>
			{/if}
		</div>
	{:else}
		<button
			class="text-tertiary hover:text-secondary text-2xs flex items-center gap-0.5"
			onclick={startAdding}
		>
			<Tag size={12} /><Plus size={8} />
		</button>
	{/if}
</div>
