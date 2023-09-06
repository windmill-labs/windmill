<script lang="ts">
	import type { ComponentCssProperty } from '../../types'
	import InputValue from './InputValue.svelte'

	export let css: ComponentCssProperty
	export let id: string
	export let key: string

	export let customCss: Record<string, ComponentCssProperty> | undefined = undefined
	export let componentStyle: Record<string, ComponentCssProperty> | undefined = undefined

	let evalClassValue: string | undefined = undefined

	function updateCss(
		componentStyle: Record<string, ComponentCssProperty> | undefined,
		customCss: Record<string, ComponentCssProperty> | undefined,
		evalClassValue: string | undefined
	) {
		const { class: componentClass, style: componentStyleValue } = componentStyle?.[key] ?? {}
		const { class: customClass, style: customStyleValue } = customCss?.[key] ?? {}

		css.class = [componentClass, customClass, evalClassValue].filter(Boolean).join(' ')
		css.style = [componentStyleValue, customStyleValue].filter(Boolean).join(';')
	}

	$: updateCss(componentStyle, customCss, evalClassValue)
</script>

{#each Object.keys(css ?? {}) as key (key)}
	{#if css.evalClass}
		<InputValue {id} bind:value={evalClassValue} input={css.evalClass} />
	{/if}
{/each}
