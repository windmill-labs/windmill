<script lang="ts">
	import { Button, Popup } from './common'
	import { Pen } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'

	import DefaultTagsInner from './DefaultTagsInner.svelte'

	let placement: 'bottom-end' | 'top-end' = 'bottom-end'
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: placement }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
>
	<svelte:fragment slot="button">
		<Button color="dark" size="xs" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center"
				><Pen size={14} /> Default Tags&nbsp;<Tooltip light
					>Scripts and steps that have not been specifically assigned tags will use a default tag
					that can be customized here</Tooltip
				></div
			>
		</Button>
	</svelte:fragment>
	<DefaultTagsInner />
</Popup> 
