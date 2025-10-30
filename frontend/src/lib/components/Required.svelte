<!-- @migration-task Error while migrating Svelte code: $$props is used together with named props in a way that cannot be automatically migrated. -->
<script lang="ts">
	export let required: boolean
	export let detail = ''
</script>

{#if required}
	<span class="text-red-500 dark:text-red-400 text-sm font-normal {$$props.class}">*</span>
{:else if detail || detail != ''}
	<span class="text-sm text-primary ml-2 font-normal {$$props.class}"
		>({detail != '' ? `${detail}` : ''})</span
	>
{/if}
