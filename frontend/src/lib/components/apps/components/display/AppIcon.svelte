<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import { AlignWrapper, InputValue } from '../helpers'
	import { loadIcon } from '../icon'

	export let id: string
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let configuration: Record<string, AppInput>
	export let customCss: ComponentCustomCSS<'container' | 'icon'> | undefined = undefined
	export let render: boolean

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	let icon: string | undefined = undefined
	let size: number
	let color: string
	let strokeWidth: number
	let iconComponent: any

	$: handleIcon(icon)

	async function handleIcon(i?: string) {
		iconComponent = i ? await loadIcon(i) : undefined
	}

	$: css = concatCustomCss($app.css?.iconcomponent, customCss)
</script>

<InputValue {id} input={configuration.icon} bind:value={icon} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.strokeWidth} bind:value={strokeWidth} />

<AlignWrapper
	{render}
	{horizontalAlignment}
	{verticalAlignment}
	class={css?.container?.class ?? ''}
	style={css?.container?.style ?? ''}
>
	{#if icon && iconComponent}
		<svelte:component
			this={iconComponent}
			size={size || 24}
			color={color || 'currentColor'}
			strokeWidth={strokeWidth || 2}
			class={css?.icon?.class ?? ''}
			style={css?.icon?.style ?? ''}
		/>
	{/if}
</AlignWrapper>
