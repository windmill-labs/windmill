<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Button, SecondsInput } from '$lib/components/common'
	import { Trash } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import type { ComponentProps } from 'svelte'

	interface Props {
		delete_after_secs: number | undefined
		btnProps?: ComponentProps<typeof Button>
	}

	let { delete_after_secs = $bindable(), btnProps }: Props = $props()

	const enabled = $derived(typeof delete_after_secs === 'number' && delete_after_secs >= 0)
</script>

<Popover
	floatingConfig={{
		middleware: [
			autoPlacement({
				allowedPlacements: ['bottom-start', 'bottom-end', 'top-start', 'top-end', 'top', 'bottom']
			})
		]
	}}
	closeButton
	contentClasses="block text-primary p-4"
>
	{#snippet trigger()}
		<Button
			nonCaptureEvent={true}
			btnClasses={enabled
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primary hover:bg-hover'}
			color="light"
			variant="contained"
			size="xs2"
			iconOnly
			startIcon={{ icon: Trash }}
			title="Delete job metadata after the job completes"
			{...btnProps}
		/>
	{/snippet}
	{#snippet content()}
		<Toggle
			checked={enabled}
			on:change={() => {
				delete_after_secs = enabled ? undefined : 0
			}}
			options={{
				right: 'Delete job metadata after the job completes'
			}}
		/>
		<div class="mt-4">
			<span class="text-xs font-bold">Delay before deletion</span>
			<SecondsInput bind:seconds={delete_after_secs} disabled={!enabled} />
			<span class="text-2xs text-tertiary block mt-1">0 = immediate</span>
		</div>
	{/snippet}
</Popover>
