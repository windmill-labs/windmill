<script lang="ts">
	import type { EvalAppInput } from '../../../inputType'

	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib } from '$lib/components/apps/utils'

	export let componentInput: EvalAppInput | undefined
	export let id: string

	const {
		onchange,
		worldStore,
		state: stateStore
	} = getContext<AppViewerContext>('AppViewerContext')

	$: extraLib =
		componentInput?.expr && $worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, id, $stateStore, false)
			: undefined

	// 	`
	// /** The current's app state */
	// const state: Record<string, any> = ${JSON.stringify(state)};`
</script>

{#if componentInput?.type === 'eval'}
	<div class="border">
		<SimpleEditor
			lang="javascript"
			bind:code={componentInput.expr}
			shouldBindKey={false}
			{extraLib}
			autoHeight
			on:change={() => {
				if (onchange) {
					onchange()
				}
			}}
		/>
	</div>
{/if}
