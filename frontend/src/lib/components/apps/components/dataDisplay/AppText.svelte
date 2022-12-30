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

	let result: string | undefined = undefined
</script>

<RunnableWrapper bind:componentInput {id} bind:result>
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		{#if result === ''}
			<div class="text-gray-400 bg-gray-100 flex justify-center items-center h-full w-full">
				No text
			</div>
		{:else}
			<div class="prose-sm">
				<SvelteMarkdown source={String(result)} />
			</div>
		{/if}
	</AlignWrapper>
</RunnableWrapper>

<style>
	.prose-sm p {
		margin: 0px !important;
	}
</style>
