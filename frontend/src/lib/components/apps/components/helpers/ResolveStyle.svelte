<script lang="ts">
	import { deepEqual } from 'fast-equals'
	import type { ComponentCssProperty } from '../../types'
	import InputValue from './InputValue.svelte'

	export let css: ComponentCssProperty
	export let id: string
	export let key: string
	export let extraKey: string = ''

	export let customCss: Record<string, ComponentCssProperty> | undefined = undefined
	export let componentStyle: Record<string, ComponentCssProperty> | undefined = undefined

	let evalClassValue: string | undefined = undefined
	let evalClassValueGlobal: string | undefined = undefined

	function updateCss(
		componentStyle: Record<string, ComponentCssProperty> | undefined,
		customCss: Record<string, ComponentCssProperty> | undefined,
		evalClassValue: string | undefined,
		evalClassValueGlobal: string | undefined
	) {
		const { class: componentClass, style: componentStyleValue } = componentStyle?.[key] ?? {}
		const { class: customClass, style: customStyleValue } = customCss?.[key] ?? {}

		const newCss = {
			class: [componentClass, customClass, evalClassValue ?? evalClassValueGlobal]
				.filter(Boolean)
				.join(' '),
			style: [componentStyleValue, customStyleValue].filter(Boolean).join(';')
		}

		if (!deepEqual(newCss, css)) {
			css = newCss
		}
	}

	// When any of the values change, update the css
	$: updateCss(componentStyle, customCss, evalClassValue, evalClassValueGlobal)

	// We need to clear the evalClassValue if the user has disabled the evalClass
	$: if (customCss?.[key]?.evalClass === undefined && evalClassValue !== undefined) {
		evalClassValue = undefined
	}

	// We need to clear the evalClassValue if the user has disabled the evalClass
	$: if (componentStyle?.[key]?.evalClass === undefined && evalClassValueGlobal !== undefined) {
		evalClassValueGlobal = undefined
	}
</script>

{#if customCss}
	{@const property = customCss[key]}
	{#if property?.evalClass}
		<InputValue
			field={key}
			key={key + extraKey + 'css'}
			{id}
			bind:value={evalClassValue}
			input={property.evalClass}
		/>
	{/if}
{/if}

{#if componentStyle}
	{@const property = componentStyle[key]}
	{#if property?.evalClass}
		<InputValue
			field={key}
			key={key + extraKey + 'cssGlobal'}
			{id}
			bind:value={evalClassValueGlobal}
			input={property.evalClass}
		/>
	{/if}
{/if}
