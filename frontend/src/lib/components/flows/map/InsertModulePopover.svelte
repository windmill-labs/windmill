<script lang="ts">
	import InsertModuleInner from './InsertModuleInner.svelte'
	import type { ComputeConfig } from 'svelte-floating-ui'

	import PopupV2 from '$lib/components/common/popup/PopupV2.svelte'
	import { flip, offset } from 'svelte-floating-ui/dom'

	// import type { Writable } from 'svelte/store'

	type Alignment = 'start' | 'end' | 'center'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	interface Props {
		allowTrigger?: boolean
		funcDesc?: string
		kind?: 'script' | 'trigger' | 'preprocessor' | 'failure'
		placement?: Placement
		disableAi?: boolean
		trigger: import('svelte').Snippet<[{ toggleOpen: () => void }]>
	}

	let {
		funcDesc = $bindable(''),
		kind = 'script',
		placement = 'bottom-center',
		disableAi = false,
		trigger,
		allowTrigger = false
	}: Props = $props()

	let floatingConfig: ComputeConfig = {
		strategy: 'fixed',
		// @ts-ignore
		placement,
		middleware: [offset(8), flip()],
		autoUpdate: true
	}

	let open = $state(false)

	$effect(() => {
		!open && (funcDesc = '')
	})
</script>

<PopupV2 {floatingConfig} bind:open target="#flow-editor">
	{#snippet button()}
		{@render trigger?.({ toggleOpen: () => (open = !open) })}
	{/snippet}
	{#snippet children({ close })}
		<InsertModuleInner
			on:close={() => close()}
			on:insert
			on:new
			on:pickFlow
			on:pickScript
			{allowTrigger}
			{kind}
			{disableAi}
		/>
	{/snippet}
</PopupV2>
