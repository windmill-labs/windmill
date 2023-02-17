<script lang="ts">
	import type { AppInput } from '../inputType'
	import type { HorizontalAlignment, VerticalAlignment } from '../types'
	import AlignWrapper from './helpers/AlignWrapper.svelte'
	import InputValue from './helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let horizontalAlignment: HorizontalAlignment | undefined = undefined
	export let verticalAlignment: VerticalAlignment | undefined = undefined
	export let position: 'horizontal' | 'vertical'
	let size = 2
	let color = '#00000060'
</script>

<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.color} bind:value={color} />

<AlignWrapper {horizontalAlignment} {verticalAlignment} class="h-full">
	<div
		class="rounded-full {position === 'horizontal' ? 'w-full' : 'h-full'}"
		style="{position === 'horizontal' ? 'height' : 'width'}: {size}px; background-color: {color}"
	/>
</AlignWrapper>
