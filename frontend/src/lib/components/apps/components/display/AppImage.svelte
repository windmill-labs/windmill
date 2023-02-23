<script lang="ts">
	import type { staticValues } from '../../editor/componentsPanel/componentStaticValues'
	import type { AppInput } from '../../inputType'
	import InputValue from '../helpers/InputValue.svelte'

	type FitOption = typeof staticValues['objectFitOptions'][number]

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['loading']

	const fit: Record<FitOption, string> = {
		cover: 'object-cover',
		contain: 'object-contain',
		fill: 'object-fill',
	}

	let source: string | undefined = undefined
	let imageFit: FitOption | undefined = undefined
	let altText: string | undefined = undefined
	let customStyles: string | undefined = undefined
</script>

<InputValue {id} input={configuration.source} bind:value={source} />
<InputValue {id} input={configuration.imageFit} bind:value={imageFit} />
<InputValue {id} input={configuration.altText} bind:value={altText} />
<InputValue {id} input={configuration.customStyles} bind:value={customStyles} />

<img
	src={source}
	alt={altText}
	style={customStyles}
	class="w-full h-full {fit[imageFit || 'cover']}"
>
