<script lang="ts">
	import Badge from './common/badge/Badge.svelte'
	import { Plus, Tag, X } from 'lucide-svelte'

	interface Props {
		labels: string[] | undefined
		onchange?: () => void
	}

	let { labels = $bindable(), onchange }: Props = $props()

	let adding = $state(false)
	let inputValue = $state('')
	let inputEl: HTMLInputElement | undefined = $state()

	function startAdding() {
		adding = true
		inputValue = ''
		setTimeout(() => inputEl?.focus(), 0)
	}

	function addLabel() {
		const value = inputValue.trim().slice(0, 50)
		if (!value) {
			adding = false
			return
		}
		if (!labels) {
			labels = []
		}
		if (!labels.includes(value)) {
			labels = [...labels, value]
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
			addLabel()
		} else if (e.key === 'Escape') {
			adding = false
		}
	}
</script>

<div class="inline-flex items-center gap-1 -mt-3 ml-0.5">
	{#each labels ?? [] as label (label)}
		<Badge color="blue" small>
			{label}
			<button class="ml-0.5 hover:text-red-500" onclick={() => removeLabel(label)}>
				<X size={10} />
			</button>
		</Badge>
	{/each}
	{#if adding}
		<input
			bind:this={inputEl}
			bind:value={inputValue}
			onkeydown={onKeydown}
			onblur={addLabel}
			class="text-xs border border-blue-300 rounded px-1.5 py-0.5 max-w-32 outline-none focus:ring-1 focus:ring-blue-400"
			placeholder="label"
		/>
	{:else}
		<button
			class="text-tertiary hover:text-secondary text-2xs flex items-center gap-0.5"
			onclick={startAdding}
		>
			<Tag size={12} /><Plus size={8} />
		</button>
	{/if}
</div>
