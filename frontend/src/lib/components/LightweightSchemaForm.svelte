<script lang="ts">
	import type { Schema } from '$lib/common'
	import { twMerge } from 'tailwind-merge'
	import LightweightArgInput from './LightweightArgInput.svelte'
	import type { ComponentCustomCSS } from './apps/types'
	import { allTrue, computeShow } from '$lib/utils'
	import { deepEqual } from 'fast-equals'

	export let css: ComponentCustomCSS<'schemaformcomponent'> | undefined = undefined

	export let schema: Schema | Record<string, any>
	export let args: Record<string, any> | undefined = undefined
	export let displayType: boolean = true
	export let largeGap: boolean = false
	export let isValid: boolean = true
	export let defaultValues: Record<string, any> = {}
	export let dynamicEnums: Record<string, any> = {}
	export let disabled: boolean = false

	let inputCheck: { [id: string]: boolean } = {}
	let errors: { [id: string]: string } = {}

	$: isValid = allTrue(inputCheck) ?? false

	$: if (args === undefined) {
		args = {}
	}

	export function invalidate(key: string, error: string) {
		inputCheck[key] = false
		errors[key] = error
	}

	export function validate(key: string) {
		inputCheck[key] = true
		errors[key] = ''
	}

	export function validateAll() {
		inputCheck = Object.fromEntries(Object.entries(inputCheck).map((x) => [x[0], true]))
		errors = Object.fromEntries(Object.entries(errors).map((x) => [x[0], '']))
	}

	let keys: string[] = Array.isArray(schema?.order)
		? schema?.order
		: Object.keys(schema?.properties ?? {})

	$: schema && reorder()

	function reorder() {
		let lkeys = Object.keys(schema?.properties ?? {})
		if (!deepEqual(schema?.order, lkeys) || !deepEqual(keys, lkeys)) {
			console.debug('reorder')
			if (schema?.order && Array.isArray(schema.order)) {
				const n = {}

				;(schema.order as string[]).forEach((x) => {
					if (schema.properties && schema.properties[x] != undefined) {
						n[x] = schema.properties[x]
					}
				})

				Object.keys(schema.properties ?? {})
					.filter((x) => !schema.order?.includes(x))
					.forEach((x) => {
						n[x] = schema.properties[x]
					})
				schema.properties = n
			}
			keys = Object.keys(schema.properties ?? {})
		}
	}
</script>

<div
	class={twMerge('w-full flex flex-col px-0.5 pb-2', largeGap ? 'gap-8' : 'gap-2')}
	on:pointerdown
>
	{#each keys as argName (argName)}
		{#if typeof args == 'object' && schema?.properties[argName] && args}
			<LightweightArgInput
				render={computeShow(argName, schema?.properties[argName].showExpr, args)}
				label={argName}
				description={schema.properties[argName].description}
				bind:value={args[argName]}
				bind:valid={inputCheck[argName]}
				bind:error={errors[argName]}
				type={schema.properties[argName].type}
				oneOf={schema.properties[argName].oneOf}
				required={schema.required?.includes(argName) ?? false}
				pattern={schema.properties[argName].pattern}
				defaultValue={defaultValues?.[argName] ?? schema.properties[argName].default}
				enum_={dynamicEnums?.[argName] ?? schema.properties[argName].enum}
				format={schema.properties[argName].format}
				contentEncoding={schema.properties[argName].contentEncoding}
				customErrorMessage={schema.properties[argName].customErrorMessage}
				properties={schema.properties[argName].properties}
				nestedRequired={schema.properties[argName].required}
				itemsType={schema.properties[argName].items}
				extra={schema.properties[argName]}
				title={schema.properties[argName].title}
				placeholder={schema.properties[argName].placeholder}
				on:inputClicked
				{displayType}
				{css}
				disabled={disabled || schema.properties[argName].disabled}
			/>
		{/if}
	{/each}
</div>
