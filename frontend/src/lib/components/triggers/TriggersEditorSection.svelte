<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { ChevronDown, Save } from 'lucide-svelte'
	import CaptureWrapper from './CaptureWrapper.svelte'
	import { capitalize } from '$lib/utils'
	import Popover from '../Popover.svelte'
	import Section from '../Section.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { type CaptureTriggerKind } from '$lib/gen'
	import type { FlowEditorContext } from '../flows/types'
	import type { TriggerContext } from '$lib/components/triggers'
	import TriggersWrapper from './TriggersWrapper.svelte'
	import { twMerge } from 'tailwind-merge'

	export let cloudDisabled: boolean
	export let triggerType: CaptureTriggerKind
	export let isFlow: boolean = false
	export let data: any = {}
	export let noSave = false
	export let isEditor: boolean = false
	export let path: string = ''
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false
	export let newItem: boolean

	const captureTypeLabels: Record<CaptureTriggerKind, string> = {
		http: 'New custom HTTP route',
		websocket: 'New websocket trigger',
		webhook: 'Webhook',
		kafka: 'New kafka trigger',
		email: 'Email trigger'
	}

	const { captureOn } = getContext<TriggerContext>('TriggerContext')

	let args: Record<string, any> = {}

	const dispatch = createEventDispatcher()

	const { flowStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext') || {}
</script>

<Section label={captureTypeLabels[triggerType]}>
	<svelte:fragment slot="action">
		<div class="flex flex-row grow w-min-0 gap-2 items-center justify-end">
			{#if isEditor}
				<Button
					size="xs2"
					on:click={() => {
						$captureOn = !$captureOn
					}}
					variant="border"
					color="light"
					endIcon={{
						icon: ChevronDown,
						classes: twMerge('transition', $captureOn ? 'rotate-180' : '')
					}}
				>
					Capture
				</Button>
			{/if}

			{#if !noSave}
				{@const disabled = newItem || cloudDisabled}
				<Popover notClickable>
					<Button
						size="xs2"
						{disabled}
						startIcon={{ icon: Save }}
						on:click={() => {
							console.log('saveTrigger', args)
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
						{:else}
							Create new {captureTypeLabels[triggerType].toLowerCase()}
						{/if}
					</svelte:fragment>
				</Popover>
			{/if}
		</div>
	</svelte:fragment>

	{#if isEditor}
		<CaptureWrapper
			{path}
			{isFlow}
			captureType={triggerType}
			{hasPreprocessor}
			{canHavePreprocessor}
			on:applyArgs
			on:addPreprocessor
			on:updateSchema={(e) => {
				const { schema, redirect } = e.detail
				$flowStore.schema = schema
				if (redirect) {
					$selectedId = 'Input'
				}
			}}
			on:saveTrigger
			bind:args
			{data}
			showCapture={$captureOn}
		/>
	{:else}
		<TriggersWrapper {path} {isFlow} {triggerType} {cloudDisabled} {args} {data} />
	{/if}
</Section>
