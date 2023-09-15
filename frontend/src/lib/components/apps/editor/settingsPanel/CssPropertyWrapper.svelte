<script lang="ts">
	import type { ComponentCssProperty } from '../../types'
	import CssProperty from '../componentsPanel/CssProperty.svelte'

	export let forceStyle: boolean = false
	export let forceClass: boolean = false
	export let id: string
	export let property: ComponentCssProperty | undefined = undefined
	export let overriden: boolean = false
	export let overridding: boolean = false
	export let wmClass: string | undefined = undefined

	function hasValues(obj: ComponentCssProperty | undefined) {
		if (!obj) return false

		return Object.values(obj).some((v) => v !== '')
	}
</script>

{#if property}
	<CssProperty
		{forceStyle}
		{forceClass}
		name={id}
		bind:value={property[id]}
		shouldDisplayLeft={hasValues(property[id])}
		on:left
		on:right
		{overriden}
		{overridding}
		{wmClass}
	/>
{/if}
