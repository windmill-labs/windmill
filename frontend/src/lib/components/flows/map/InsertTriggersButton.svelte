<script lang="ts">
	import { Cross, Zap } from 'lucide-svelte'
	import StepGenQuick from '$lib/components/copilot/StepGenQuick.svelte'
	import FlowInputsQuick from '../content/FlowInputsQuick.svelte'
	import type { FlowModule } from '$lib/gen'
	import ToggleHubWorkspaceQuick from '$lib/components/ToggleHubWorkspaceQuick.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ComputeConfig } from 'svelte-floating-ui'
	import PopupV2 from '$lib/components/common/popup/PopupV2.svelte'
	import { flip, offset } from 'svelte-floating-ui/dom'
	import { createEventDispatcher } from 'svelte'

	// import type { Writable } from 'svelte/store'

	export let index: number = 0
	export let funcDesc = ''
	export let modules: FlowModule[] = []
	export let disableAi = false
	export let kind: 'script' | 'trigger' | 'preprocessor' | 'failure' = 'script'

	type Alignment = 'start' | 'end' | 'center'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	export let placement: Placement = 'bottom-center'

	let floatingConfig: ComputeConfig = {
		strategy: 'fixed',
		// @ts-ignore
		placement,
		middleware: [offset(8), flip()],
		autoUpdate: true
	}
	$: !open && (funcDesc = '')
	let preFilter: 'all' | 'workspace' | 'hub' = 'all'
	let loading = false
	let small = false
	let open = false
	let dispatch = createEventDispatcher()

	$: small = kind === 'preprocessor' || kind === 'failure'
</script>

<!-- <Menu transitionDuration={0} pointerDown bind:show={open} noMinW {placement} let:close> -->

<!-- {floatingConfig}
floatingClasses="mt-2"
containerClasses="border rounded-lg shadow-lg  bg-surface"
noTransition
shouldUsePortal={true} -->

<PopupV2 {floatingConfig} bind:open let:close target="#flow-editor">
	<svelte:fragment let:pointerdown let:pointerup slot="button">
		<button
			title={`Add ${
				kind === 'failure'
					? ' failure module '
					: kind === 'preprocessor'
					? 'preprocessor step'
					: kind === 'trigger'
					? 'trigger'
					: 'step'
			}`}
			id={`flow-editor-add-step-${index}`}
			type="button"
			class={twMerge(
				'w-5 h-5 flex items-center justify-center',
				'outline-[1px] outline dark:outline-gray-500 outline-gray-300',
				'text-secondary',
				'bg-surface focus:outline-none hover:bg-surface-hover rounded '
			)}
			on:pointerdown|preventDefault|stopPropagation={pointerdown}
			on:pointerup={pointerup}
		>
			{#if kind === 'trigger'}
				<Zap size={12} />
			{:else}
				<Cross size={12} />
			{/if}
		</button>
	</svelte:fragment>
	<!-- FOO -->
	<div
		id="flow-editor-insert-module"
		class="flex flex-col h-[400px] {small ? 'w-[450px]' : 'w-[650px]'}  pt-1 pr-1 pl-1 gap-1.5"
		on:wheel={(e) => {
			e.stopPropagation()
		}}
		role="none"
	>
		<div class="flex flex-row items-center gap-2">
			<StepGenQuick {disableAi} on:insert bind:funcDesc {preFilter} {loading} />

			<ToggleHubWorkspaceQuick bind:selected={preFilter} />
		</div>

		<div class="flex flex-row grow min-h-0">
			<div class="flex-none flex flex-col text-xs text-primary">
				<slot name="topLevelNodes" {close} />
			</div>

			<FlowInputsQuick
				selectedKind={'trigger'}
				bind:loading
				filter={funcDesc}
				{modules}
				{index}
				{disableAi}
				{funcDesc}
				{kind}
				on:close={() => {
					close(null)
				}}
				on:new={(event) => {
					dispatch('new', event.detail)
					close(null)
				}}
				on:pickScript={(event) => {
					dispatch('pickScript', event.detail)
					close(null)
				}}
				{preFilter}
				{small}
			/>
		</div>
	</div>
</PopupV2>
