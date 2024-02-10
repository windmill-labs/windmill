<script lang="ts">
	import { Button, Popup } from './common'
	import { Pen } from 'lucide-svelte'

	import Tooltip from './Tooltip.svelte'
	import AssignableTagsInner from './AssignableTagsInner.svelte'

	export let placement: 'bottom-end' | 'top-end' = 'bottom-end'
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: placement }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
>
	<svelte:fragment slot="button">
		<Button color="dark" size="xs" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center"
				><Pen size={14} /> Custom Tags&nbsp;<Tooltip light
					>Tags are assigned to scripts and flows. Workers only accept jobs that correspond to their
					worker tags. Scripts have a default tag based on the language they are in but users can
					choose to override their tags with custom ones. This editor allow you to set the custom
					tags one can override the scripts and flows with.</Tooltip
				></div
			>
		</Button>
	</svelte:fragment>
	<AssignableTagsInner on:refresh />
</Popup>
