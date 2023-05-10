<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Required from './Required.svelte'
	import { addWhitespaceBeforeCapitals, capitalize } from '$lib/utils'

	export let label: string
	export let format: string = ''
	export let contentEncoding = ''
	export let type: string | undefined = undefined
	export let required = false
	export let displayType: boolean = true
	export let labelClass: string = ''
</script>

<div class="inline-flex flex-row items-center truncated">
	<span class={twMerge('font-semibold', labelClass)}>
		{addWhitespaceBeforeCapitals(label.replace(/_/g, ' ')).split(' ').map(capitalize).join(' ')}
	</span>
	<Required {required} class="!ml-0" />

	{#if format && format != ''}
		<span class="text-sm italic ml-1 text-indigo-800">
			({format})
		</span>
	{:else if displayType}
		<span class="text-sm italic ml-1 text-indigo-800">
			({type ?? 'any'}{contentEncoding && contentEncoding != ''
				? `, encoding: ${contentEncoding}`
				: ''})
		</span>
	{/if}
</div>
