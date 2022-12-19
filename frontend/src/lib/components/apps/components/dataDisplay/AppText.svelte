<script lang="ts">
	import SvelteMarkdown from 'svelte-markdown'
	import type { AppInput } from '../../inputType'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	export const staticOutputs: string[] = ['result', 'loading']

	let result: string = ''
</script>

<RunnableWrapper bind:componentInput {id} bind:result autoRefresh={false}>
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		{#if result === ''}
			<div class="text-gray-400 bg-gray-100 flex justify-center items-center h-full w-full">
				No text
			</div>
		{:else}
			<SvelteMarkdown source={String(result)} />
		{/if}
	</AlignWrapper>
</RunnableWrapper>
