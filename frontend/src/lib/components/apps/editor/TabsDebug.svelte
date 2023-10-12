<script lang="ts">
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import Button from '$lib/components/common/button/Button.svelte'
	import { ChevronDown } from 'lucide-svelte'

	export let tabs: any[] = []
	export let id: string

	export let isConditionalDebugMode: boolean = false

	const { componentControl } = getContext<AppViewerContext>('AppViewerContext')

	let isManuallySelected: boolean = false
	let renderCount = 0
</script>

<ButtonDropdown hasPadding={false}>
	<svelte:fragment slot="buttonReplacement">
		<Button
			title={isConditionalDebugMode ? 'Debug conditions' : 'Debug tabs'}
			variant="contained"
			color={isManuallySelected ? 'red' : 'light'}
			size="xs2"
			nonCaptureEvent
		>
			<ChevronDown size={16} />
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="items">
		{#each tabs ?? [] as { }, index}
			<MenuItem
				on:click={() => {
					$componentControl?.[id]?.setTab?.(index)
					isManuallySelected = true
				}}
			>
				<div
					class={classNames(
						'!text-tertiary text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
					)}
				>
					{#if index === tabs.length - 1}
						{isConditionalDebugMode ? `Debug default condition` : `Debug tab ${index + 1}`}
					{:else}
						{`Debug ${isConditionalDebugMode ? 'condition' : 'tab'} ${index + 1}`}
					{/if}
				</div>
			</MenuItem>
		{/each}
		<MenuItem
			on:click={() => {
				$componentControl?.[id]?.setTab?.(-1)
				isManuallySelected = false
			}}
		>
			<div
				class={classNames(
					'!text-red-600 text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
				)}
			>
				{`Reset debug mode`}
			</div>
		</MenuItem>
	</svelte:fragment>
</ButtonDropdown>
