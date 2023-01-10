<script lang="ts">
	import type { AppInput } from '../../inputType'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let configuration: Record<string, AppInput>

	export const staticOutputs: string[] = ['result', 'loading']

	let extraStyle: string | undefined = undefined
	let result: string | undefined = undefined

	let style: 'Title' | 'Subtitle' | 'Body' | 'Caption' | 'Label' | undefined = undefined

	function getComponent() {
		switch (style) {
			case 'Title':
				return 'h1'
			case 'Subtitle':
				return 'h3'
			case 'Body':
				return 'p'
			case 'Caption':
				return 'p'
			case 'Label':
				return 'label'
			default:
				return 'p'
		}
	}

	function getClasses() {
		switch (style) {
			case 'Caption':
				return 'text-sm italic text-gray-500'
			case 'Label':
				return 'font-semibold text-sm'
			default:
				return ''
		}
	}

	let component = 'p'
	let classes = ''
	$: style && (component = getComponent())
	$: style && (classes = getClasses())
</script>

<InputValue {id} input={configuration.extraStyle} bind:value={extraStyle} />
<InputValue {id} input={configuration.style} bind:value={style} />

<RunnableWrapper flexWrap bind:componentInput {id} bind:result>
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		{#if !result || result === ''}
			<div class="text-gray-400 bg-gray-100 flex justify-center items-center h-full w-full">
				No text
			</div>
		{:else}<svelte:element
				this={component}
				class="whitespace-pre-wrap {classes}"
				style={extraStyle}
			>
				{String(result)}
			</svelte:element>
		{/if}
	</AlignWrapper>
</RunnableWrapper>
