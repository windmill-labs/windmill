<script lang="ts">

	interface Props {
		required: boolean;
		detail?: string;
		class?: string;
	}

	let { required, detail = '', class: className = '' }: Props = $props();
	
</script>

{#if required}
	<span class="text-red-500 dark:text-red-400 text-sm font-normal {className}">*</span>
{:else if detail || detail != ''}
	<span class="text-sm text-primary ml-2 font-normal {className}"
		>({detail != '' ? `${detail}` : ''})</span
	>
{/if}
