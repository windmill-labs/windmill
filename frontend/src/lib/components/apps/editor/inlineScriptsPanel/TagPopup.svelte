<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Tag } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import type { ComponentProps } from 'svelte'
	import WorkerTagSelect from '$lib/components/WorkerTagSelect.svelte'

	interface Props {
		tag: string | undefined
		btnProps?: ComponentProps<typeof Button>
	}

	let { tag = $bindable(), btnProps }: Props = $props()

	$effect(() => {
		if (tag === '') tag = undefined
	})
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
	contentClasses="block text-primary p-3 w-56"
>
	{#snippet trigger()}
		<Button
			nonCaptureEvent={true}
			btnClasses={tag
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primary hover:bg-hover'}
			color="light"
			variant="contained"
			size="xs2"
			iconOnly
			startIcon={{ icon: Tag }}
			title={tag ? `Worker tag: ${tag}` : 'Assign a worker tag'}
			{...btnProps}
		/>
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col gap-2">
			<span class="text-xs font-bold">Worker tag</span>
			<span class="text-2xs text-tertiary">
				Assign a worker tag to run this inline script on a specific worker group. Leave empty to use
				the language default.
			</span>
			<WorkerTagSelect bind:tag noLabel placeholder="lang default" />
		</div>
	{/snippet}
</Popover>
