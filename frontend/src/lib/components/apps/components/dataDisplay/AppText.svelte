<script lang="ts">
	import SvelteMarkdown from 'svelte-markdown'
	import type { AppInput } from '../../inputType'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	export const staticOutputs: string[] = ['result']

	let result: string = ''
</script>

<RunnableWrapper {componentInput} {id} bind:result>
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		<SvelteMarkdown source={String(result)} />
	</AlignWrapper>
</RunnableWrapper>
