<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { CircleStop, Play, Save } from 'lucide-svelte'
	import CaptureWrapper from './CaptureWrapper.svelte'
	import { capitalize } from '$lib/utils'
	import Popover from '../Popover.svelte'
	import Section from '../Section.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { type CaptureTriggerKind } from '$lib/gen'
	import type { FlowEditorContext } from '../flows/types'
	import Toggle from '../Toggle.svelte'

	export let cloudDisabled: boolean
	export let captureType: CaptureTriggerKind
	export let isFlow: boolean = false

	let args: Record<string, any> = {}
	let captureMode = false
	let active = false

	const dispatch = createEventDispatcher()

	const { flowStore, initialPath, pathStore, selectedId } =
		getContext<FlowEditorContext>('FlowEditorContext')

	$: newItem = initialPath === ''

	let handleCapture: (() => Promise<void>) | undefined
</script>

<Section label="New Route">
	<svelte:fragment slot="action">
		<div class=" flex flex-row grow w-min-0 gap-2 px-2 items-center justify-between">
			<Toggle bind:checked={captureMode} options={{ left: 'Capture' }} size="xs" />
			<div class="flex flex-row gap-2">
				{#if captureMode}
					<Button
						size="xs2"
						on:click={() => handleCapture?.()}
						disabled={false}
						color={active ? 'red' : 'dark'}
						startIcon={{ icon: active ? CircleStop : Play }}
					>
						Capture
					</Button>
				{/if}

				{#if newItem || cloudDisabled}
					<Popover notClickable>
						<Button
							size="xs2"
							disabled
							startIcon={{ icon: Save }}
							iconOnly
							wrapperClasses="h-full"
						/>
						<svelte:fragment slot="text">
							{#if newItem}
								Deploy the runnable to enable trigger creation
							{:else if cloudDisabled}
								{capitalize(captureType)} triggers are disabled in the multi-tenant cloud
							{/if}
						</svelte:fragment>
					</Popover>
				{:else}
					<Button
						size="xs2"
						on:click={() => {
							dispatch('saveTrigger', {
								config: { ...args, captureMode }
							})
						}}
						startIcon={{ icon: Save }}
						iconOnly
					/>
				{/if}
			</div>
		</div>
	</svelte:fragment>

	<CaptureWrapper
		path={$pathStore}
		{isFlow}
		captureType="http"
		hasPreprocessor={!!$flowStore.value.preprocessor_module}
		canHavePreprocessor
		on:applyArgs
		on:addPreprocessor={async () => {
			console.log('dbg add preprocessor')
			$selectedId = 'preprocessor'
		}}
		on:updateSchema={(e) => {
			const { schema, redirect } = e.detail
			$flowStore.schema = schema
			if (redirect) {
				//tabSelected = 'input'
				console.log('dbg redirect')
			}
		}}
		bind:args
		{captureMode}
		bind:handleCapture
		bind:active
	/>
</Section>
