<script lang="ts">
	import { getContext } from 'svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import type { AppEditorContext, ComponentInputsSpec } from '../../types'
	import AlignWrapper from './AlignWrapper.svelte'

	export let componentInputs: ComponentInputsSpec
	export let horizontalAlignement: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignement: 'top' | 'center' | 'bottom' | undefined = undefined

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	export const staticOutputs: string[] = []
</script>

{#if $worldStore && componentInputs?.content?.type === 'static'}
	<div class="prose">
		<AlignWrapper {horizontalAlignement} {verticalAlignement}>
			<SvelteMarkdown source={componentInputs?.content?.value} />
		</AlignWrapper>
	</div>
{/if}
