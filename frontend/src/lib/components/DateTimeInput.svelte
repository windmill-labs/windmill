<script lang="ts">
	import Toggle from './Toggle.svelte'

	export let value: string | undefined = undefined

	let input: HTMLInputElement
	export function focus() {
		input?.focus()
	}

	let hasTime: boolean = Boolean(value)

	$: if (value && hasTime !== value.includes('T')) {
		value = hasTime ? `${value}T00:00` : value.slice(0, 10)
	}
</script>

<div class="flex flex-row gap-1 items-center w-full">
	{#if hasTime}
		<input type="datetime-local" bind:value bind:this={input} {...$$restProps} />
	{:else}
		<input type="date" bind:value bind:this={input} {...$$restProps} />
	{/if}
	<Toggle
		bind:checked={hasTime}
		options={{ right: 'Set time' }}
		size="xs"
		textClass="whitespace-nowrap"
	/>
</div>
