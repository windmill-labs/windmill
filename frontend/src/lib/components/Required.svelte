<script lang="ts">
	export let required: boolean
	export let detail = ''
</script>

{#if required}
	<span class="text-red-700 dark:text-red-400 font-normal {$$props.class}">*</span>
{:else if detail || detail != ''}
	<span class="text-sm text-gray-500 ml-2 font-normal {$$props.class}"
		>({detail != '' ? `${detail}` : ''})</span
	>
{/if}
