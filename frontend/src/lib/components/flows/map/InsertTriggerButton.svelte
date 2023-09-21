<script lang="ts">
	import { Menu } from '$lib/components/common'
	import { faBolt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import StepGen from '$lib/components/copilot/StepGen.svelte'
	import type { FlowModule } from '$lib/gen'

	const dispatch = createEventDispatcher()
	export let open: boolean | undefined = undefined
	export let index: number
	export let funcDesc = ''
	export let modules: FlowModule[]

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
		class="text-primary bg-surface border mx-0.5 rotate-180 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
	>
		<Icon data={faBolt} scale={0.8} />
	</button>
	<StepGen {index} bind:funcDesc bind:open {close} {modules} trigger />
	{#if funcDesc.length === 0}
		<div class="font-mono divide-y text-xs w-full text-secondary">
			<button
				class="w-full text-left py-2 px-3 hover:bg-surface-hover"
				on:pointerdown={() => {
					close()
					dispatch('new', 'trigger')
				}}
				role="menuitem"
				tabindex="-1"
			>
				<Icon data={faBolt} scale={0.8} class="mr-2" />
				Trigger
			</button>
		</div>
	{/if}
</Menu>
