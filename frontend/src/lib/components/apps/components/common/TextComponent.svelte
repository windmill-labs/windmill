<script lang="ts">
	import SvelteMarkdown from 'svelte-markdown'
	import type { ComponentInputsSpec } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import ComponentInputValue from '../helpers/ComponentInputValue.svelte'

	export let componentInputs: ComponentInputsSpec
	export let horizontalAlignement: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignement: 'top' | 'center' | 'bottom' | undefined = undefined

	export const staticOutputs: string[] = []

	let contentValue: string = ''
</script>

<ComponentInputValue input={componentInputs.content} bind:value={contentValue} />

<AlignWrapper {horizontalAlignement} {verticalAlignement}>
	<SvelteMarkdown source={contentValue} />
</AlignWrapper>
