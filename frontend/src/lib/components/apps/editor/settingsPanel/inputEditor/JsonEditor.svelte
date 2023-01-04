<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	export let code: string
	export let value: any

	function parseJson() {
		try {
			value = JSON.parse(code)
			error = ''
		} catch (e) {
			error = e.message
		}
	}
	let error = ''
	$: code && parseJson()
</script>

<div class="flex flex-col w-full">
	<div class="border border-gray-300 w-full">
		<SimpleEditor autoHeight lang="json" bind:code />
	</div>
	{#if error != ''}
		<span class="text-red-600 text-xs">{error}</span>
	{/if}
</div>
