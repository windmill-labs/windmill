<script lang="ts">
	import type { AppInput } from '../../inputType'
	import { AlignWrapper, InputValue, RunnableWrapper } from '../helpers'
	import { toKebabCase } from '../../utils'

	export let id: string
	export let componentInput: AppInput | undefined
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined
	export const staticOutputs: string[] = ['result', 'loading']

	let result: string | undefined = undefined
	let icon: string | undefined = undefined
	let size: number
	let color: string
	let strokeWidth: number
	let iconComponent: any

	$: icon && loadIcon(icon)

	async function loadIcon(name: string) {
		try {
			if(name) {
				name = toKebabCase(name).replace(/([a-z])(\d)/i, '$1-$2')
				iconComponent = (await import(
					`../../../../../../node_modules/lucide-svelte/dist/svelte/icons/${name}.svelte`
				)).default
			} else {
				iconComponent = undefined
			}
		} catch (error) {
			console.error(error);
			iconComponent = undefined
		}
	}
</script>

<InputValue {id} input={configuration.icon} bind:value={icon} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.strokeWidth} bind:value={strokeWidth} />

<RunnableWrapper autoRefresh flexWrap bind:componentInput {id} bind:initializing bind:result>
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
</RunnableWrapper>