<script lang="ts">
	import type { ComponentCssProperty } from '../../types'
	import CssProperty from '../componentsPanel/CssProperty.svelte'

	interface Props {
		forceStyle?: boolean
		forceClass?: boolean
		id: string
		property?: ComponentCssProperty | undefined
		overriden?: boolean
		overridding?: boolean
		wmClass?: string | undefined
	}

	let {
		forceStyle = false,
		forceClass = false,
		id,
		property = $bindable(undefined),
		overriden = false,
		overridding = false,
		wmClass = undefined
	}: Props = $props()

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
