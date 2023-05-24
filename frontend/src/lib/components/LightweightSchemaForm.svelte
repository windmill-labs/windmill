<script lang="ts">
	import type { Schema } from '$lib/common'
	import { twMerge } from 'tailwind-merge'
	import LightweightArgInput from './LightweightArgInput.svelte'
	import type { ComponentCustomCSS } from './apps/types'
	import { allTrue } from '$lib/utils'

	export let css: ComponentCustomCSS<'schemaformcomponent'> | undefined = undefined

	export let schema: Schema
	export let args: Record<string, any> | undefined = undefined
	export let displayType: boolean = true
	export let largeGap: boolean = false
	export let isValid: boolean = true

	let inputCheck: { [id: string]: boolean } = {}
	$: isValid = allTrue(inputCheck) ?? false

	$: if (args === undefined) {
		args = {}
	}
</script>

<div class={twMerge('w-full flex flex-col overflow-auto', largeGap ? 'gap-8' : 'gap-2')}>
	{#each Object.keys(schema.properties ?? {}) as argName (argName)}
		<div>
			{#if typeof args == 'object' && schema?.properties[argName] && args}
				<LightweightArgInput
					label={argName}
					description={schema.properties[argName].description}
					bind:value={args[argName]}
					bind:valid={inputCheck[argName]}
					type={schema.properties[argName].type}
					required={schema.required?.includes(argName) ?? false}
					pattern={schema.properties[argName].pattern}
					defaultValue={schema.properties[argName].default}
					enum_={schema.properties[argName].enum}
					format={schema.properties[argName].format}
					contentEncoding={schema.properties[argName].contentEncoding}
					properties={schema.properties[argName].properties}
					itemsType={schema.properties[argName].items}
					extra={schema.properties[argName]}
					on:inputClicked
					{displayType}
					{css}
				/>
			{/if}
		</div>
	{/each}
</div>
