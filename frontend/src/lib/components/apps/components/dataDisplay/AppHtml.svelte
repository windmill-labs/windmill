<script lang="ts">
	import type { AppInput } from '../../inputType'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let configuration: Record<string, AppInput>

	export const staticOutputs: string[] = ['result', 'loading']

	let result: string | undefined = undefined
</script>

<RunnableWrapper bind:componentInput {id} bind:result>
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		<div
			on:pointerdown={(e) => {
				e?.preventDefault()
			}}
			class="h-full relative w-full overflow-hidden"
		>
			{@html result ?? 'No html'}
		</div>
	</AlignWrapper>
</RunnableWrapper>
