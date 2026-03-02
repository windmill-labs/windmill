<script lang="ts">
	export let required: boolean
	export let detail = ''

	let className: string = ''
	export { className as class }
</script>

{#if required}
	<span class="text-red-500 dark:text-red-400 text-sm font-normal {className}">*</span>
{:else if detail || detail != ''}
	<span class="text-sm text-primary ml-2 font-normal {className}"
		>({detail != '' ? `${detail}` : ''})</span
	>
{/if}
