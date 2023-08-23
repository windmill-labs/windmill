<script lang="ts">
	import type { EvalV2AppInput } from '../../../inputType'

	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib } from '$lib/components/apps/utils'
	import { parseOutputs } from '$lib/infer'
	import { deepEqual } from 'fast-equals'

	export let componentInput: EvalV2AppInput | undefined
	export let id: string

	const { onchange, worldStore, state, app } = getContext<AppViewerContext>('AppViewerContext')

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
		inferDeps(componentInput.expr)
	}
	if (componentInput && componentInput.connections == undefined) {
		componentInput.connections = []
	}
	async function inferDeps(code: string) {
		const outputs = await parseOutputs(code, true)
		if (outputs && componentInput) {
			const noutputs = outputs
				.filter(
					([key, id]) =>
						key == 'row' || key == 'iter' || id in ($worldStore?.outputsById[key] ?? {})
				)
				.map(([key, id]) => ({
					componentId: key,
					id: id
				}))
			if (!deepEqual(noutputs, componentInput.connections)) {
				componentInput.connections = noutputs
				$app = $app
			}
		}
	}
</script>

{#if componentInput?.type === 'evalv2'}
	<div class="border">
		<SimpleEditor
			lang="javascript"
			bind:code={componentInput.expr}
			shouldBindKey={false}
			{extraLib}
			autoHeight
			on:change={async (e) => {
				if (onchange) {
					onchange()
				}
				inferDeps(e.detail.code)
			}}
		/>
	</div>
	{#if componentInput.connections.length > 0}
		<div class="mt-2">
			<div class="text-sm font-medium text-secondary">Automatically connected to</div>
			<div class="mt-1 flex flex-wrap gap-1">
				{#each componentInput.connections as connection (connection.componentId + '-' + connection.id)}
					<span class="inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border"
						>{connection.componentId + '.' + connection.id}</span
					>
				{/each}
			</div>
		</div>
	{/if}
{/if}
