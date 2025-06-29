<script lang="ts">
	import { deepEqual } from 'fast-equals'
	import type { ComponentCssProperty } from '../../types'
	import InputValue from './InputValue.svelte'
	import { untrack } from 'svelte'

	interface Props {
		css: ComponentCssProperty
		id: string
		key: string
		extraKey?: string
		customCss?: Record<string, ComponentCssProperty> | undefined
		componentStyle?: Record<string, ComponentCssProperty> | undefined
	}

	let {
		css = $bindable(),
		id,
		key,
		extraKey = '',
		customCss = undefined,
		componentStyle = undefined
	}: Props = $props()

	let evalClassValue: string | undefined = $state(undefined)
	let evalClassValueGlobal: string | undefined = $state(undefined)

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
	$effect(() => {
		;[componentStyle, customCss, evalClassValue, evalClassValueGlobal]
		untrack(() => updateCss(componentStyle, customCss, evalClassValue, evalClassValueGlobal))
	})

	// We need to clear the evalClassValue if the user has disabled the evalClass
	$effect(() => {
		if (customCss?.[key]?.evalClass === undefined && evalClassValue !== undefined) {
			evalClassValue = undefined
		}
	})

	// We need to clear the evalClassValue if the user has disabled the evalClass
	$effect(() => {
		if (componentStyle?.[key]?.evalClass === undefined && evalClassValueGlobal !== undefined) {
			evalClassValueGlobal = undefined
		}
	})
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
