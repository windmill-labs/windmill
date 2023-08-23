<script lang="ts">
	import type { EvalV2AppInput } from '../../../inputType'

	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib } from '$lib/components/apps/utils'
	import { inferDeps } from '../../appUtilsInfer'

	export let componentInput: EvalV2AppInput | undefined
	export let id: string

	const { onchange, worldStore, state, app } = getContext<AppViewerContext>('AppViewerContext')

	let editor: SimpleEditor
	export function setCode(code: string) {
		editor?.setCode(code)
	}

	$: extraLib =
		componentInput?.expr && $worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, id, $state, false)
			: undefined

	if (
		componentInput &&
		componentInput.expr &&
		componentInput.expr != '' &&
		componentInput.connections == undefined
	) {
		inferDepsFromCode(componentInput.expr)
	}
	if (componentInput && componentInput.connections == undefined) {
		componentInput.connections = []
	}

	function inferDepsFromCode(code: string) {
		if (componentInput) {
			inferDeps(code, $worldStore.outputsById, componentInput, app)
		}
	}
</script>

{#if componentInput?.type === 'evalv2'}
	<div class="border">
		<SimpleEditor
			bind:this={editor}
			lang="javascript"
			bind:code={componentInput.expr}
			shouldBindKey={false}
			{extraLib}
			autoHeight
			on:change={async (e) => {
				if (onchange) {
					onchange()
				}
				inferDepsFromCode(e.detail.code)
			}}
		/>
	</div>
	{#if componentInput.connections.length > 0}
		<div class="flex flex-wrap gap-2 items-center">
			<div class="text-2xs text-tertiary">Re-evaluated on changes to:</div>
			<div class="flex flex-wrap gap-1">
				{#each componentInput.connections as connection (connection.componentId + '-' + connection.id)}
					<span class="inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border"
						>{connection.componentId + '.' + connection.id}</span
					>
				{/each}
			</div>
		</div>
	{/if}
{/if}
