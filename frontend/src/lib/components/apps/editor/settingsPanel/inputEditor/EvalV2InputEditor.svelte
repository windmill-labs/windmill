<script lang="ts">
	import type { EvalV2AppInput } from '../../../inputType'

	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '$lib/components/apps/types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib, ctxRegex } from '$lib/components/apps/utils'
	import { inferDeps } from '../../appUtilsInfer'
	import { Maximize2, Shield, X } from 'lucide-svelte'
	import { Drawer } from '$lib/components/common'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Toggle from '$lib/components/Toggle.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import Popover from '$lib/components/Popover.svelte'

	export let componentInput: EvalV2AppInput | undefined
	export let id: string
	export let field: string
	export let fixedOverflowWidgets: boolean = true
	export let acceptSelf: boolean = false
	export let recomputeOnInputChanged = true
	export let showOnDemandOnlyToggle = false
	export let securedContext = false

	const {
		onchange,
		worldStore,
		state: stateStore,
		app
	} = getContext<AppViewerContext>('AppViewerContext')
	const { evalPreview } = getContext<AppEditorContext>('AppEditorContext')

	let editor: SimpleEditor
	export function setCode(code: string) {
		editor?.setCode(code)
	}

	$: extraLib =
		componentInput?.expr && $worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, acceptSelf ? '' : id, $stateStore, false)
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
	let focus = false
</script>

{#if componentInput?.type === 'evalv2'}
	{#if fullscreen}
		<Drawer
			placement="bottom"
			on:close={() => (fullscreen = false)}
			open
			offset={zIndexes.monacoEditor}
		>
			<Splitpanes horizontal class="h-full">
				<Pane size={50}>
					<SimpleEditor
						loadAsync
						class="h-full w-full"
						bind:this={editor}
						lang="javascript"
						bind:code={() => componentInput.expr ?? '', (e) => (componentInput.expr = e)}
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
				</Pane>
				<Pane size={50}>
					<div class="relative w-full">
						<div
							class="p-1 !text-2xs absolute border border-l bg-surface w-full z-[5000] overflow-auto"
						>
							<pre class="text-tertiary"
								>{JSON.stringify($evalPreview[`${id}.${field}`] ?? null, null, 4) ?? 'null'}</pre
							>
						</div>
					</div>
				</Pane>
			</Splitpanes>
		</Drawer>
	{/if}
	<div class="border relative">
		{#if !fullscreen}
			<SimpleEditor
				loadAsync
				small
				bind:this={editor}
				lang="javascript"
				bind:code={() => componentInput.expr ?? '', (e) => (componentInput.expr = e)}
				shouldBindKey={false}
				{extraLib}
				autoHeight
				{fixedOverflowWidgets}
				on:focus={() => {
					focus = true
				}}
				on:blur={() => {
					focus = false
				}}
				on:change={async (e) => {
					if (onchange) {
						onchange()
					}
					inferDepsFromCode(e.detail.code)
				}}
			/>
			{#if securedContext && componentInput?.expr?.match(ctxRegex)}
				<div class="border bg-surface absolute top-0.5 right-8 p-0.5">
					<Popover notClickable>
						<Shield size={12} />
						<svelte:fragment slot="text">
							This context variable is securely provided by the backend and cannot be altered by
							users
						</svelte:fragment>
					</Popover>
				</div>
			{/if}
			<button
				title="Open in drawer"
				class="border bg-surface absolute hover:text-primary top-0 right-2 p-0.5 text-secondary"
				on:click={() => (fullscreen = true)}><Maximize2 size={10} /></button
			>
			{#if focus}
				<div class="relative w-full">
					<div
						class="p-1 !text-2xs absolute rounded-b border-b border-r border-l bg-surface w-full z-[5000] overflow-auto"
					>
						<!-- <div class="text-tertiary absolute top-0 right-0 !text-2xs">{id}.{field}</div> -->
						<div class="float-right text-tertiary cursor-pointer"><X size={14} /></div>
						<pre class="text-tertiary"
							>{JSON.stringify($evalPreview[`${id}.${field}`] ?? null, null, 4) ?? 'null'}</pre
						>
					</div>
				</div>
			{/if}
		{:else}
			<pre class="text-small border px-2">{componentInput.expr}</pre>
		{/if}
	</div>
	<!-- <div class="relative">
		<div class="absolute top-0 left-0 z-[1000]"> -->

	{#if componentInput?.expr && componentInput.expr != '' && componentInput.expr
			.trim()
			.startsWith('{')}
		<div class="text-2xs text-red-500"
			><pre class="font-mono inline">{'{...}'}</pre> are not valid js expressions, prefix with
			<pre class="font-mono inline text-2xs">{'return '}</pre>
			or surround with <pre class="font-mono text-2xs inline">{'()'}</pre></div
		>
	{/if}

	{#if componentInput.connections?.length > 0 && recomputeOnInputChanged}
		<div class="flex flex-wrap gap-2 items-center">
			{#if showOnDemandOnlyToggle}
				<Toggle
					size="xs"
					color="blue"
					disabled={!recomputeOnInputChanged}
					checked={!componentInput.onDemandOnly}
					on:change={() => {
						if (componentInput) {
							componentInput.onDemandOnly = !componentInput.onDemandOnly
							if (!componentInput.onDemandOnly) {
								delete componentInput.onDemandOnly
							}
						}
					}}
				/>
			{/if}
			<div class="text-2xs text-tertiary"
				>{componentInput.onDemandOnly ? 'NOT' : ''} Re-evaluated on changes to:</div
			>
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
