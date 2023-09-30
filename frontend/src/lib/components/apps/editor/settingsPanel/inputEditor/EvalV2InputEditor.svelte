<script lang="ts">
	import type { EvalV2AppInput } from '../../../inputType'

	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib } from '$lib/components/apps/utils'
	import { inferDeps } from '../../appUtilsInfer'
	import { Maximize2 } from 'lucide-svelte'
	import { Drawer } from '$lib/components/common'

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

	let fullscreen = false
</script>

{#if componentInput?.type === 'evalv2'}
	{#if fullscreen}
		<Drawer placement="bottom" on:close={() => (fullscreen = false)} open>
			<SimpleEditor
				class="h-full w-full"
				bind:this={editor}
				lang="javascript"
				bind:code={componentInput.expr}
				shouldBindKey={false}
				fixedOverflowWidgets={false}
				{extraLib}
				on:change={async (e) => {
					if (onchange) {
						onchange()
					}
					inferDepsFromCode(e.detail.code)
				}}
			/>
		</Drawer>
	{/if}
	<div class="border relative">
		{#if !fullscreen}
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
			<button
				class="border bg-surface absolute top-0.5 right-2 p-0.5"
				on:click={() => (fullscreen = true)}><Maximize2 size={12} /></button
			>
		{:else}
			<pre class="text-small border px-2">{componentInput.expr}</pre>
		{/if}
	</div>

	{#if componentInput?.expr && componentInput.expr != '' && componentInput.expr
			.trim()
			.startsWith('{')}
		<div class="text-2xs text-red-500"
			><pre class="font-mono inline">{'{...}'}</pre> are not valid js expressions, prefix with
			<pre class="font-mono inline text-2xs">{'return '}</pre>
			or surround with <pre class="font-mono text-2xs inline">{'()'}</pre></div
		>
	{/if}
	{#if componentInput.connections?.length > 0}
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
