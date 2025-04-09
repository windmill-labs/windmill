<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Save } from 'lucide-svelte'
	import { capitalize } from '$lib/utils'
	import Popover from '../Popover.svelte'
	import Section from '../Section.svelte'
	import { createEventDispatcher } from 'svelte'
	import { type CaptureTriggerKind } from '$lib/gen'
	import TriggersWrapper from './TriggersWrapper.svelte'

	export let cloudDisabled: boolean
	export let triggerType: CaptureTriggerKind
	export let isFlow: boolean = false
	export let data: any = {}
	export let noSave = false
	export let path: string = ''
	export let newItem: boolean
	export let openForm: boolean = false
	export let alwaysOpened: boolean = false

	let args: Record<string, any> = {}
	$: collapsed = !openForm

	const dispatch = createEventDispatcher()
</script>

<Section label={triggerType} collapsable={!alwaysOpened} bind:collapsed>
	<svelte:fragment slot="action">
		{#if !collapsed || alwaysOpened}
			<div class="flex flex-row grow w-min-0 gap-2 items-center justify-end">
				{#if !noSave}
					{@const disabled = newItem || cloudDisabled}
					<Popover notClickable>
						<Button
							size="xs2"
							{disabled}
							startIcon={{ icon: Save }}
							on:click={() => {
								dispatch('saveTrigger', {
									config: args
								})
							}}
						>
							Save
						</Button>
						<svelte:fragment slot="text">
							{#if disabled}
								{#if newItem}
									Deploy the runnable to enable trigger creation
								{:else if cloudDisabled}
									{capitalize(triggerType)} triggers are disabled in the multi-tenant cloud
								{/if}
							{/if}
						</svelte:fragment>
					</Popover>
				{/if}
			</div>
		{/if}
	</svelte:fragment>

	<slot name="description" />

	<TriggersWrapper {path} {isFlow} {triggerType} {cloudDisabled} {args} {data} />
</Section>
