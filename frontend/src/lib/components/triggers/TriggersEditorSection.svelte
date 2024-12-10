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
	import ConnectionIndicator from '$lib/components/common/alert/ConnectionIndicator.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { onDestroy } from 'svelte'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import TriggersWrapper from './TriggersWrapper.svelte'

	export let cloudDisabled: boolean
	export let triggerType: CaptureTriggerKind
	export let isFlow: boolean = false
	export let data: any = {}
	export let noSave = false
	export let isEditor: boolean = false
	export let path: string = ''

	const captureTypeLabels: Record<CaptureTriggerKind, string> = {
		http: 'Route',
		websocket: 'Websocket',
		webhook: 'Webhook',
		kafka: 'Kafka',
		email: 'Email'
	}

	const { captureOn } = getContext<TriggerContext>('TriggerContext')

	let args: Record<string, any> = {}

	let captureActive = false
	let connectionInfo:
		| {
				status: 'connected' | 'disconnected' | 'error'
				message?: string
		  }
		| undefined = undefined

	const dispatch = createEventDispatcher()

	const { flowStore, initialPath, selectedId } =
		getContext<FlowEditorContext>('FlowEditorContext') || {}

	let newItem = false
	$: initialPath && (newItem = initialPath === '')

	let handleCapture: (() => Promise<void>) | undefined

	onDestroy(() => {
		$captureOn = false
	})

	$: $captureOn && handleCapture?.()
</script>

<Section label={`New ${captureTypeLabels[triggerType]} Trigger`}>
	<svelte:fragment slot="action">
		<div class="flex flex-row grow w-min-0 gap-2 px-2 items-center justify-end">
			{#if isEditor}
				<ConnectionIndicator {connectionInfo} />

				<AnimatedButton animate={captureActive} baseRadius="6px">
					<Button
						size="xs2"
						on:click={() => {
							handleCapture?.()
						}}
						variant="border"
						disabled={false}
						color="light"
						startIcon={{ icon: captureActive ? CircleStop : Play }}
						btnClasses={captureActive ? 'text-blue-500 hover:text-blue-500' : ''}
					>
						Live test
					</Button>
				</AnimatedButton>
			{/if}

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
						{:else}
							Create new {captureTypeLabels[triggerType]} trigger from prototype
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
			on:saveTrigger
			bind:args
			bind:handleCapture
			bind:captureActive
			{data}
			bind:connectionInfo
		/>
	{:else}
		<TriggersWrapper {path} {isFlow} {triggerType} {cloudDisabled} {args} {data} />
	{/if}
</Section>
