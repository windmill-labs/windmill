<script lang="ts">
	import Toggle from './Toggle.svelte'

	export let name: string
	export let value: any

	$: enabled = value != undefined
</script>

<div class="flex flex-col gap-1">
	<label class="text-sm font-medium text-gray-700">{name}</label><Toggle
		checked={enabled}
		on:change={(e) => {
			if (e.detail) {
				value = { id: '', secret: '' }
			} else {
				value = undefined
			}
		}}
	/>
	{#if enabled}
		<label class="block pb-2">
			<span class="text-primary font-semibold text-sm">Client Id</span>
			<input type="text" placeholder="Client Id" bind:value={value['id']} />
		</label>
		<label class="block pb-2">
			<span class="text-primary font-semibold text-sm">Client Secret</span>
			<input type="text" placeholder="Client Secret" bind:value={value['secret']} />
		</label>
	{/if}
</div>
