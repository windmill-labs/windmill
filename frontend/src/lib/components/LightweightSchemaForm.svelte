<script lang="ts">
	import type { Schema } from '$lib/common'
	import LightweightArgInput from './LightweightArgInput.svelte'

	export let schema: Schema
	export let args: Record<string, any> | undefined = undefined

	$: if (args === undefined) {
		args = {}
	}
</script>

<div class="w-full">
	{#each Object.keys(schema.properties ?? {}) as argName (argName)}
		<div>
			{#if typeof args == 'object' && schema?.properties[argName] && args}
				<LightweightArgInput
					label={argName}
					description={schema.properties[argName].description}
					bind:value={args[argName]}
					type={schema.properties[argName].type}
					required={schema.required.includes(argName)}
					pattern={schema.properties[argName].pattern}
					defaultValue={schema.properties[argName].default}
					enum_={schema.properties[argName].enum}
					format={schema.properties[argName].format}
					contentEncoding={schema.properties[argName].contentEncoding}
					properties={schema.properties[argName].properties}
					itemsType={schema.properties[argName].items}
					extra={schema.properties[argName]}
				/>
			{/if}
		</div>
	{/each}
</div>
