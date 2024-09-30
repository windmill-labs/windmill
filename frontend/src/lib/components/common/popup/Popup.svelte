<script lang="ts">
	import { Popover, PopoverButton, PopoverPanel, Transition } from '@rgossiaux/svelte-headlessui'
	import ConditionalPortal from '../drawer/ConditionalPortal.svelte'
	import { createFloatingActions, type ComputeConfig } from 'svelte-floating-ui'

	export let floatingConfig: ComputeConfig = {
		strategy: 'absolute',
		placement: 'bottom-start'
	}

	export let containerClasses: string = 'rounded-lg shadow-md border p-4 bg-surface'
	export let floatingClasses: string = ''
	const [floatingRef, floatingContent] = createFloatingActions(floatingConfig)

	export let blockOpen = false
	export let shouldUsePortal: boolean = true
	export let target: string | HTMLElement | undefined = undefined
	export let noTransition = false
</script>

<Popover on:close class="leading-none">
	<PopoverButton>
		<div use:floatingRef>
			<slot name="button" />
		</div>
	</PopoverButton>
	<ConditionalPortal condition={shouldUsePortal} {target}>
		<div use:floatingContent class={`z5000 ${floatingClasses}`}>
			{#if !noTransition}
				<Transition
					show={blockOpen || undefined}
					enter="transition ease-out duration-200"
					enterFrom="opacity-0 translate-y-1"
					enterTo="opacity-100 translate-y-0"
					leave="transition ease-in duration-150"
					leaveFrom="opacity-100 translate-y-0"
					leaveTo="opacity-0 translate-y-1"
				>
					<PopoverPanel let:close static={blockOpen}>
						<div class={containerClasses}>
							<slot {close} />
						</div>
					</PopoverPanel>
				</Transition>
			{:else}
				<PopoverPanel focus={false} let:close static={blockOpen}>
					<div class={containerClasses}>
						<slot {close} />
					</div>
				</PopoverPanel>
			{/if}
		</div>
	</ConditionalPortal>
</Popover>
