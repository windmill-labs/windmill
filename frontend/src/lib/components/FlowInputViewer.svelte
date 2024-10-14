<script lang="ts">
	import type { Schema } from '$lib/common'

	import FieldHeader from './FieldHeader.svelte'

	export let schema: Schema | { [key: string]: unknown } | undefined
</script>

<ul class="my-2">
	{#if schema == undefined}
		<li class="text-secondary text-xs italic mb-4">No inputs</li>
	{:else}
		{#each Object.entries(schema.properties ?? {}) as [inp, v]}
			<li class="list-disc flex flex-row items-center">
				<FieldHeader
					label={inp}
					required={Array.isArray(schema.required) && schema.required?.includes(inp)}
					type={v?.type}
					contentEncoding={v?.contentEncoding}
					format={v?.format}
				/><span class="ml-4 mt-1 text-xs"
					>{v?.default != undefined && v?.default != ''
						? 'default: ' + JSON.stringify(v?.default)
						: ''}</span
				>
			</li>
		{/each}
	{/if}
</ul>
