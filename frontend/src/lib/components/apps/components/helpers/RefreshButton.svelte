<script lang="ts">
	import { Button } from '$lib/components/common'
	import { LoaderIcon, RefreshCw } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'
	import type { AppViewerContext } from '../../types'

	export let id: string
	export let loading: boolean

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
</script>

<Popover>
	<Button
		startIcon={{
			icon: loading ? LoaderIcon : RefreshCw,
			classes: twMerge(
				loading ? 'animate-spin text-blue-800' : '',
				'transition-all text-gray-500 dark:text-white'
			)
		}}
		color="light"
		size="xs2"
		btnClasses={twMerge(loading ? ' bg-blue-100 dark:bg-blue-400' : '', 'transition-all')}
		on:click={() => {
			$runnableComponents[id]?.cb?.map((cb) => cb())
		}}
		iconOnly
	/>
	<svelte:fragment slot="text">
		{#if loading}
			Refreshing...
		{:else}
			Refresh
		{/if}
	</svelte:fragment>
</Popover>
