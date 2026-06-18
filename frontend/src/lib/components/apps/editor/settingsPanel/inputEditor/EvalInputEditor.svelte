<script lang="ts">
	import type { EvalAppInput } from '../../../inputType'

	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib } from '$lib/components/apps/utils'

	interface Props {
		componentInput: EvalAppInput | undefined
		id: string
	}

	let { componentInput = $bindable(), id }: Props = $props()

	const {
		onchange,
		worldStore,
		state: stateStore
	} = getContext<AppViewerContext>('AppViewerContext')

	let extraLib = $derived(
		componentInput?.expr && $worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, id, $stateStore, false)
			: undefined
	)

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
