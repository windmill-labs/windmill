<script lang="ts">
	import type { AppInput } from '../../inputType'
	import { AlignWrapper, InputValue, RunnableWrapper } from '../helpers'
	import { loadIcon } from '../icon'

	export let id: string
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = []

	let icon: string | undefined = undefined
	let size: number
	let color: string
	let strokeWidth: number
	let iconComponent: any

	$: icon && handleIcon()

	async function handleIcon() {
		if (icon) {
			iconComponent = await loadIcon(icon)
		}
	}
</script>

<InputValue {id} input={configuration.icon} bind:value={icon} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.strokeWidth} bind:value={strokeWidth} />

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	{#if iconComponent}
		<svelte:component
			this={iconComponent}
			size={size || 24}
			color={color || 'currentColor'}
			strokeWidth={strokeWidth || 2}
		/>
	{/if}
</AlignWrapper>
