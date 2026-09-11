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
	}

	let { tag = $bindable(), input, workspace = undefined }: Props = $props()

	let defaultTag = $derived(getDefaultDbTag(input))
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	{#snippet trigger()}
		<Button
			size="xs"
			color="light"
			startIcon={{ icon: Tag }}
			nonCaptureEvent
			title="Worker tag the database jobs run on"
		>
			{tag ?? 'Worker tag'}
		</Button>
	{/snippet}
	{#snippet content()}
		<div class="p-4 w-96">
			<DbWorkerTagPicker bind:tag {defaultTag} {workspace} />
		</div>
	{/snippet}
</Popover>
