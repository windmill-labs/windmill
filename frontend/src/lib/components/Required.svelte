<script lang="ts">
	export let required: boolean
	export let detail = ''
</script>

{#if required}
	<span class="text-red-700 font-normal {$$props.class}">*</span>
{:else}
	<span class="text-sm text-gray-500 ml-2 font-normal {$$props.class}"
		>(optional{detail != '' ? `, ${detail}` : ''})</span
	>
{/if}
