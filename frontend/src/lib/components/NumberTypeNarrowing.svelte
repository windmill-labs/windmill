<script lang="ts">
	import Label from './Label.svelte'
	import Toggle from './Toggle.svelte'

	export let min: number | undefined
	export let max: number | undefined
	export let currency: string | undefined

	let minChecked: boolean = min != undefined
	let maxChecked: boolean = max != undefined
</script>

<div class="my-2" />

<div class="flex flex-col gap-2">
	<div class="flex gap-2">
		<Toggle bind:checked={minChecked} options={{ right: 'min' }} />
		<input type="number" bind:value={min} disabled={!minChecked} />
	</div>
	<div class="flex gap-2">
		<Toggle bind:checked={maxChecked} options={{ right: 'max' }} />
		<input type="number" bind:value={max} disabled={!maxChecked} />
	</div>
	<Label label="Currency">
		<select class="mt-1" bind:value={currency}>
			<option value={undefined}> No currency </option>
			{#each ['CHF', 'EUR', 'USD'] as c}
				<option value={c}>{c}</option>
			{/each}
		</select>
	</Label>
</div>
