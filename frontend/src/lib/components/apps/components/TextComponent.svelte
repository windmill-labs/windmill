<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppEditorContext, InputsSpec } from '../types'

	export let inputs: InputsSpec
	export let horizontalAlignement: 'left' | 'center' | 'right'
	export let verticalAlignement: 'top' | 'center' | 'bottom'

	function tailwindHorizontalAlignement(horizontalAlignement) {
		switch (horizontalAlignement) {
			case 'left':
				return 'justify-start'
			case 'center':
				return 'justify-center'
			case 'right':
				return 'justify-end'
		}
	}

	function tailwindVerticalAlignement(verticalAlignement) {
		switch (verticalAlignement) {
			case 'top':
				return 'items-start'
			case 'center':
				return 'items-center'
			case 'bottom':
				return 'items-end'
		}
	}

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	export const staticOutputs: string[] = []
	$: x = classNames(
		'flex',
		tailwindHorizontalAlignement(horizontalAlignement),
		tailwindVerticalAlignement(verticalAlignement)
	)
</script>

{#if $worldStore && inputs?.content.type === 'static'}
	<div class={x}>{inputs?.content?.value}</div>
{/if}
