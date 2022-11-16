<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, InputsSpec } from '../../types'
	import AlignWrapper from './AlignWrapper.svelte'

	export let inputs: InputsSpec
	export let horizontalAlignement: 'left' | 'center' | 'right' | undefined
	export let verticalAlignement: 'top' | 'center' | 'bottom' | undefined

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	export const staticOutputs: string[] = []
</script>

{#if $worldStore && inputs?.content.type === 'static'}
	<AlignWrapper {horizontalAlignement} {verticalAlignement}>
		{inputs?.content?.value}
	</AlignWrapper>
{/if}
