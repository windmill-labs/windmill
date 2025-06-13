<script lang="ts">
	import { Button } from '$lib/components/common'
	import { LoaderIcon, RefreshCw, RefreshCwOff } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'
	import type { AppViewerContext, CancelablePromise } from '../../types'

	interface Props {
		id: string
		loading: boolean
	}

	let { id, loading }: Props = $props()

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	let buttonHover = $state(false)
	let cancelCallbacks: CancelablePromise<void>[] | undefined = $state(undefined)
</script>

<Popover>
	<Button
		on:mouseenter={() => (buttonHover = true)}
		on:mouseleave={() => (buttonHover = false)}
		startIcon={{
			icon: loading ? (!buttonHover ? LoaderIcon : RefreshCwOff) : RefreshCw,
			classes: twMerge(
				loading && !buttonHover ? 'animate-spin text-blue-800' : '',
				'transition-all text-gray-500 dark:text-white'
			)
		}}
		color="light"
		size="xs2"
		btnClasses={twMerge(loading ? ' bg-blue-100 dark:bg-blue-400' : '', 'transition-all')}
		on:click={() => {
			if (buttonHover && loading) {
				cancelCallbacks?.forEach((cb) => cb.cancel())
			} else {
				cancelCallbacks = $runnableComponents[id]?.cb?.map((cb) => cb())
			}
		}}
		iconOnly
	/>
	{#snippet text()}
		{#if loading}
			{#if buttonHover}
				Stop Refreshing
			{:else}
				Refreshing...
			{/if}
		{:else}
			Refresh
		{/if}
	{/snippet}
</Popover>
