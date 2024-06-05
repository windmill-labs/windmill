<script lang="ts">
	import CurrencyInput from '$lib/components/apps/components/inputs/currency/CurrencyInput.svelte'
	import { SecondsInput } from '$lib/components/common'
	import Range from '$lib/components/Range.svelte'

	export let value: number | undefined
	export let min: number | undefined
	export let max: number | undefined
	export let seconds: number | undefined
	export let currency: string | undefined
	export let currencyLocale: string | undefined
	export let autofocus: boolean | null = false
	export let disabled: boolean = false
	export let valid: boolean = true
	export let defaultValue: string | undefined = undefined
	export let placeholder: string | undefined = undefined
	export let ignoreValueUndefined = false
</script>

{#if min != undefined && max != undefined}
	<Range bind:value {min} {max} />
{:else if seconds !== undefined}
	<SecondsInput bind:seconds={value} on:focus />
{:else if currency}
	<CurrencyInput
		inputClasses={{
			formatted: 'px-2 w-full py-1.5 text-black dark:text-white',
			wrapper: 'w-full windmillapp',
			formattedZero: 'text-black dark:text-white'
		}}
		noColor
		bind:value
		{currency}
		locale={currencyLocale ?? 'en-US'}
	/>
{:else}
	<div class="relative w-full">
		<!-- svelte-ignore missing-declaration -->
		<!-- svelte-ignore a11y-autofocus -->
		<input
			{autofocus}
			on:focus
			on:blur
			{disabled}
			type="number"
			on:keydown={() => {
				ignoreValueUndefined = true
			}}
			class={valid
				? ''
				: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'}
			placeholder={placeholder ?? defaultValue ?? ''}
			bind:value
			{min}
			{max}
		/>
	</div>
{/if}
