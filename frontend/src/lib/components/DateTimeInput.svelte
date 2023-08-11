<script lang="ts">
	import Toggle from './Toggle.svelte'

	export let value: string | undefined = undefined

	let input: HTMLInputElement
	export function focus() {
		input?.focus()
	}

	let hasTime: boolean = Boolean(value)
</script>

<div class="flex flex-row gap-1 items-center w-full">
	{#if hasTime}
		<input type="datetime-local" bind:value bind:this={input} on:keydown {...$$restProps} />
	{:else}
		<input type="date" bind:value bind:this={input} on:keydown {...$$restProps} />
	{/if}
	<Toggle
		bind:checked={hasTime}
		options={{ right: 'Set time' }}
		on:change={(e) => {
			const checked = e.detail
			value = value ? value.slice(0, 10) + (checked ? 'T00:00' : '') : value
		}}
		size="xs"
		textClass="whitespace-nowrap"
	/>
</div>
