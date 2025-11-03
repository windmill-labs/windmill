<script lang="ts">
	import InsertModuleInner from './InsertModuleInner.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	import type { Placement } from '@floating-ui/core'

	interface Props {
		allowTrigger?: boolean
		funcDesc?: string
		kind?: 'script' | 'trigger' | 'preprocessor' | 'failure'
		placement?: Placement
		disableAi?: boolean
		trigger: import('svelte').Snippet
		gutter?: number
	}

	let {
		funcDesc = $bindable(''),
		kind = 'script',
		disableAi = false,
		trigger: triggerSnippet,
		allowTrigger = false,
		gutter = 0
	}: Props = $props()

	let popover: Popover

	$effect(() => {
		!popover?.isOpened() && (funcDesc = '')
	})
</script>

<Popover
	bind:this={popover}
	portal="#flow-editor"
	contentClasses="p-2 max-w-lg h-[400px] !resize bg-surface"
	class="inline-block"
	usePointerDownOutside
	floatingConfig={{
		placement: 'bottom',
		strategy: 'absolute',
		gutter,
		overflowPadding: 16,
		flip: true,
		fitViewport: true,
		overlap: false
	}}
>
	{#snippet trigger()}
		{@render triggerSnippet?.()}
	{/snippet}
	{#snippet content({ close })}
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
</Popover>
