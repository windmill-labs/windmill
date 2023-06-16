<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Required from './Required.svelte'
	import { capitalize } from '$lib/utils'

	export let label: string
	export let format: string = ''
	export let contentEncoding = ''
	export let type: string | undefined = undefined
	export let required = false
	export let displayType: boolean = true
	export let labelClass: string = ''
	export let prettify = false
</script>

<div class="inline-flex flex-row items-center truncated">
	<span class={twMerge('font-semibold', labelClass)}>
		{#if prettify}
			{label.replace(/_/g, ' ').split(' ').map(capitalize).join(' ')}
		{:else}
			{label}
		{/if}
	</span>
	<Required {required} class="!ml-0" />

	{#if displayType}
		{#if format && format != ''}
			<span class="text-sm italic ml-1 text-indigo-800 dark:text-indigo-400">
				({format})
			</span>
		{:else}
			<span class="text-sm italic ml-1 text-indigo-800 dark:text-indigo-400">
				({type ?? 'any'}{contentEncoding && contentEncoding != ''
					? `, encoding: ${contentEncoding}`
					: ''})
			</span>
		{/if}
	{/if}
</div>
