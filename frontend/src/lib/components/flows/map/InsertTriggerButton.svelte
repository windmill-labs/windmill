<script lang="ts">
	import { Menu } from '$lib/components/common'
	import { createEventDispatcher } from 'svelte'
	import StepGen from '$lib/components/copilot/StepGen.svelte'
	import type { FlowModule } from '$lib/gen'
	import { Zap } from 'lucide-svelte'

	const dispatch = createEventDispatcher()
	export let open: boolean | undefined = undefined
	export let index: number
	export let funcDesc = ''
	export let modules: FlowModule[]
	export let disableAi = false

	$: !open && (funcDesc = '')
</script>

<Menu
	transitionDuration={0}
	pointerDown
	bind:show={open}
	noMinW
	placement="bottom-center"
	let:close
>
	<button
		title="Add a Trigger"
		slot="trigger"
		type="button"
		class="text-primary bg-surface border-[1px] mx-[1px] border-gray-300 dark:border-gray-500 rotate-180 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
	>
		<Zap size={14} />
	</button>
	{#if !disableAi}
		<StepGen {index} bind:funcDesc bind:open {close} {modules} trigger />
	{/if}
	{#if funcDesc.length === 0}
		<div class="font-mono divide-y text-xs w-full text-secondary whitespace-nowrap">
			<button
				class="w-full text-left py-2 px-3 hover:bg-surface-hover flex flex-row items-center gap-2"
				on:pointerdown={() => {
					close()
					dispatch('new', 'trigger')
				}}
				role="menuitem"
				tabindex="-1"
			>
				<Zap size={14} />Trigger
			</button>
		</div>
	{/if}
</Menu>
