<script lang="ts">
	import { Button } from './common'
	import { Pen } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'
	import Popover from './meltComponents/Popover.svelte'

	import DefaultTagsInner from './DefaultTagsInner.svelte'

	export let defaultTagPerWorkspace: boolean | undefined = undefined
	export let defaultTagWorkspaces: string[] | undefined = undefined

	let placement: 'bottom-end' | 'top-end' = 'bottom-end'
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: placement }} contentClasses="p-4">
	<svelte:fragment slot="trigger">
		<Button color="dark" size="xs" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center"
				><Pen size={14} /> Default tags&nbsp;<Tooltip light
					>Scripts and steps that have not been specifically assigned tags will use a default tag
					that can be customized here</Tooltip
				></div
			>
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<DefaultTagsInner bind:defaultTagPerWorkspace bind:defaultTagWorkspaces />
	</svelte:fragment>
</Popover>
