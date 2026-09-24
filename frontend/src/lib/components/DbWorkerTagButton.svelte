<script lang="ts">
	import { Tag } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import DbWorkerTagPicker from './DbWorkerTagPicker.svelte'
	import { getDefaultDbTag } from './dbOps'
	import type { DbInput } from './dbTypes'

	interface Props {
		/** Tag override; undefined runs the database's jobs on their native tag. */
		tag: string | undefined
		input: DbInput
		/** Workspace the custom tags are read from; defaults to the navigation one. */
		workspace?: string
		/** Render no button: the picker is opened through `open`, and pops up where this is placed. */
		hideTrigger?: boolean
		open?: boolean
	}

	let {
		tag = $bindable(),
		input,
		workspace = undefined,
		hideTrigger = false,
		open = $bindable(false)
	}: Props = $props()

	let defaultTag = $derived(getDefaultDbTag(input))
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	bind:isOpen={open}
	class={hideTrigger ? 'w-0 h-0 overflow-hidden' : undefined}
	triggerAttrs={hideTrigger ? { tabindex: -1, 'aria-hidden': true } : undefined}
>
	{#snippet trigger()}
		{#if !hideTrigger}
			<Button
				size="xs"
				color="light"
				startIcon={{ icon: Tag }}
				nonCaptureEvent
				title="Worker tag the database jobs run on"
			>
				{tag ?? 'Worker tag'}
			</Button>
		{/if}
	{/snippet}
	{#snippet content()}
		<div class="p-4 w-96">
			<DbWorkerTagPicker bind:tag {defaultTag} {workspace} />
		</div>
	{/snippet}
</Popover>
