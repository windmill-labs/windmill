<script lang="ts">
	import { fly } from 'svelte/transition'
	import { secondaryMenuLeft, secondaryMenuRight } from './'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import DocLink from '../DocLink.svelte'

	interface Props {
		right: boolean
	}

	let { right }: Props = $props()

	let secondaryMenu = right ? secondaryMenuRight : secondaryMenuLeft
	let width: number | undefined = $state()
</script>

<div
	bind:clientWidth={width}
	class={`absolute inset-0 overflow-hidden w-full`}
	style={`z-index: ${zIndexes.secondaryMenu}`}
	class:pointer-events-none={!$secondaryMenu.isOpen}
>
	{#if $secondaryMenu.isOpen && $secondaryMenu.component}
		<div
			transition:fly|local={{ duration: 300, x: right ? width : -width, y: 0, opacity: 1 }}
			class="flex flex-col w-full h-full bg-surface"
		>
			<div
				class="flex justify-between {right ? '' : 'flex-row-reverse'} items-center gap-1 px-3 py-2"
			>
				<CloseButton on:close={() => secondaryMenu?.close()} />
				{#if $secondaryMenu?.props?.type === 'style'}
					<div class="flex flex-row items-center gap-1">
						<div class="text-xs font-bold"> Style Panel</div>
						<DocLink
							docLink="https://www.windmill.dev/docs/apps/app_configuration_settings/app_styling"
						/>
					</div>
				{/if}
			</div>
			<div class="relative h-full overflow-y-auto">
				{#if typeof $secondaryMenu.component === 'string'}
					{@html $secondaryMenu.component}
				{:else}
					{@const SvelteComponent = $secondaryMenu.component}
					<SvelteComponent {...$secondaryMenu.props} />
				{/if}
			</div>
		</div>
	{/if}
</div>
