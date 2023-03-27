<script lang="ts">
	import Required from './Required.svelte'

	export let label: string
	export let format: string = ''
	export let contentEncoding = ''
	export let type: string | undefined = undefined
	export let required = false
</script>

<div class="inline-flex flex-row items-center truncated">
	<span class="font-semibold">
		{label}
	</span>
	<Required {required} class="!ml-0" />

	{#if format && format != ''}
		<span class="text-sm italic ml-1 text-indigo-800">
			({format})
		</span>
	{:else}
		<span class="text-sm italic ml-1 text-indigo-800">
			({type ?? 'any'}{contentEncoding && contentEncoding != ''
				? `, encoding: ${contentEncoding}`
				: ''})</span
		>
	{/if}
</div>
