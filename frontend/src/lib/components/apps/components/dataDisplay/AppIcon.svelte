<script lang="ts">
	import type { AppInput } from '../../inputType'
	import { AlignWrapper, InputValue, RunnableWrapper } from '../helpers'
	import * as icons from 'lucide-svelte'
	import { toPascalCase } from '../../utils'

	export let id: string
	export let componentInput: AppInput | undefined
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined
	export const staticOutputs: string[] = ['result', 'loading']

	let iconName: string | undefined = undefined
	let result: string | undefined = undefined
	let icon: any
	let size: number
	let color: string
	let strokeWidth: number

	$: iconName && loadIcon(iconName)

	async function loadIcon(name: string) {
		try {
			if(name) {
				icon = icons[toPascalCase(name)]
			} else {
				icon = undefined
			}
			// const {Smile} = await import(`lucide-svelte`)
			// icon = Smile
			// icon = (await import(`lucide-svelte`))[name]
			// icon = await import(`lucide-svelte/dist/svelte/icons/smile.svelte`)
		} catch (error) {
			console.log(error);
			icon = undefined
		}
	}
</script>

<InputValue {id} input={configuration.iconName} bind:value={iconName} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.strokeWidth} bind:value={strokeWidth} />

<RunnableWrapper autoRefresh flexWrap bind:componentInput {id} bind:initializing bind:result>
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		{#if icon}
			<svelte:component
				this={icon}
				size={size || 24}
				color={color || 'currentColor'}
				strokeWidth={strokeWidth || 2}
			/>
		{/if}
	</AlignWrapper>
</RunnableWrapper>