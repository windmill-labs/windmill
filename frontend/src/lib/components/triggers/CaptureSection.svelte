<script context="module" lang="ts">
	export type CaptureInfo = {
		active: boolean
		hasPreprocessor: boolean
		canHavePreprocessor: boolean
		isFlow: boolean
		path: string
		connectionInfo: ConnectionInfo | undefined
	}
</script>

<script lang="ts">
	import { slide } from 'svelte/transition'
	import AnimatedButton from '../common/button/AnimatedButton.svelte'
	import Button from '../common/button/Button.svelte'
	import { CircleStop } from 'lucide-svelte'
	import ConnectionIndicator, {
		type ConnectionInfo
	} from '../common/alert/ConnectionIndicator.svelte'
	import CaptureTable from './CaptureTable.svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import type { CaptureTriggerKind } from '$lib/gen'
	import CaptureIcon from './CaptureIcon.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { convert } from '@redocly/json-to-json-schema'
	import { sendUserToast } from '$lib/toast'
	export let disabled: boolean
	export let captureType: CaptureTriggerKind
	export let captureInfo: CaptureInfo
	export let captureTable: CaptureTable | undefined

	const dispatch = createEventDispatcher<{
		captureToggle: undefined
		updateSchema: { schema: any; redirect: boolean; args: any }
	}>()

	onDestroy(() => {
		if (captureInfo.active) {
			dispatch('captureToggle')
		}
	})

	function handleUpdateSchema(e: any) {
		dispatch('updateSchema', {
			schema: schemaFromPayload(e.detail.payloadData),
			redirect: e.detail.redirect,
			args: e.detail.args ? e.detail.payloadData : undefined
		})
	}

	function schemaFromPayload(payload: any) {
		const parsed = JSON.parse(JSON.stringify(payload))

		if (!parsed) {
			sendUserToast('Invalid Schema', true)
			return
		}

		return { required: [], properties: {}, ...convert(parsed) }
	}
</script>

<div transition:slide class="pb-12">
	<div class="border p-4 rounded-lg">
		<div class="flex flex-col gap-1 mb-4">
			<div class="flex flex-row items-center justify-start gap-1">
				<AnimatedButton animate={captureInfo.active} baseRadius="6px" wrapperClasses="ml-[-2px]">
					<Button
						size="xs2"
						on:click={() => dispatch('captureToggle')}
						variant="border"
						{disabled}
						color="light"
						startIcon={{ icon: captureInfo.active ? CircleStop : CaptureIcon }}
						btnClasses={captureInfo.active ? 'text-blue-500 hover:text-blue-500' : ''}
					>
						{captureInfo.active ? 'Stop' : 'Start capturing'}
					</Button>
				</AnimatedButton>

				{#if captureInfo.active}
					<ConnectionIndicator connectionInfo={captureInfo.connectionInfo} />
				{:else}
					<Tooltip>
						Start capturing to test your runnables with real data. Once active, all incoming
						payloads will be captured and displayed below, allowing you to test your runnables
						effectively.
					</Tooltip>
				{/if}
			</div>

			{#if disabled}
				<div class="text-sm font-normal text-red-600 dark:text-red-400" transition:slide>
					Enter a valid configuration to start capturing.
				</div>
			{/if}
		</div>

		{#if $$slots.default}
			<div class:opacity-50={disabled || !captureInfo.active} class="flex flex-col gap-4 mb-4">
				<slot />
			</div>
		{/if}

		<CaptureTable
			bind:this={captureTable}
			{captureType}
			hasPreprocessor={captureInfo.hasPreprocessor}
			canHavePreprocessor={captureInfo.canHavePreprocessor}
			isFlow={captureInfo.isFlow}
			path={captureInfo.path}
			canEdit={true}
			on:applyArgs
			on:updateSchema={handleUpdateSchema}
			on:addPreprocessor
			captureActive={captureInfo.active}
		/>
	</div>
</div>
