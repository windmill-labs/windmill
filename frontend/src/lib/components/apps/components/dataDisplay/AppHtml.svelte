<script lang="ts">
	import type { AppInput } from '../../inputType'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>

	export const staticOutputs: string[] = ['result', 'loading']

	let result: string | undefined = undefined
	let h: number | undefined = undefined
	let w: number | undefined = undefined
</script>

<div
	on:pointerdown={(e) => {
		e?.preventDefault()
	}}
	class="h-full w-full"
	bind:clientHeight={h}
	bind:clientWidth={w}
>
	<RunnableWrapper autoRefresh flexWrap bind:componentInput {id} bind:result>
		{#key result}
			<iframe
				frameborder="0"
				style="height: {h}px; width: {w}px"
				class="p-0"
				title="sandbox"
				srcdoc={result ? '<scr' + `ipt src="/tailwind.css"></script>` + result : 'No html'}
			/>
		{/key}
	</RunnableWrapper>
</div>
