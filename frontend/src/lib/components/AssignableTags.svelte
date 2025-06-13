<script lang="ts">
	import { Button } from './common'
	import { Pen } from 'lucide-svelte'
	import Popover from './meltComponents/Popover.svelte'

	import Tooltip from './Tooltip.svelte'
	import AssignableTagsInner from './AssignableTagsInner.svelte'

	interface Props {
		placement?: 'bottom-end' | 'top-end'
		color?: 'nord' | 'dark'
		disabled?: boolean
		showWorkspaceRestriction?: boolean
	}

	let {
		placement = 'bottom-end',
		color = 'dark',
		disabled = false,
		showWorkspaceRestriction = false
	}: Props = $props()
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: placement }}
	{disabled}
	closeButton
	usePointerDownOutside
>
	{#snippet trigger()}
		<Button {color} size="xs" nonCaptureEvent={true} {disabled}>
			<div class="flex flex-row gap-1 items-center"
				><Pen size={14} /> Custom tags&nbsp;<Tooltip light
					>Tags are assigned to scripts and flows. Workers only accept jobs that correspond to their
					worker tags. Scripts have a default tag based on the language they are in but users can
					choose to override their tags with custom ones. This editor allow you to set the custom
					tags one can override the scripts and flows with.</Tooltip
				></div
			>
		</Button>
	{/snippet}
	{#snippet content()}
		<AssignableTagsInner {showWorkspaceRestriction} on:refresh />
	{/snippet}
</Popover>
