<script lang="ts">
	import type { EvalAppInput } from '../../../inputType'

	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib } from '$lib/components/apps/utils'

	export let componentInput: EvalAppInput | undefined
	export let id: string
	export let hasRows: boolean = false

	const { onchange, worldStore, state } = getContext<AppViewerContext>('AppViewerContext')

	$: componentInput && onchange?.()

	$: extraLib =
		componentInput?.expr && $worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, id, hasRows, $state, false)
			: undefined
</script>

{#if componentInput?.type === 'eval'}
	<div class="border border-gray-300 ">
		<SimpleEditor
			lang="javascript"
			bind:code={componentInput.expr}
			shouldBindKey={false}
			{extraLib}
			autoHeight
		/>
	</div>
{/if}
