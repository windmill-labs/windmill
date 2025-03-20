<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Pin, History, Pen, Check, X } from 'lucide-svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import ToggleSimple from '$lib/components/meltComponents/ToggleSimple.svelte'
	import StepHistory from './StepHistory.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { createEventDispatcher } from 'svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import type { Job } from '$lib/gen'

	export let jsonData = {}
	export let prefix: string = ''
	export let allowCopy: boolean = false
	export let isConnecting: boolean = false
	export let mock:
		| {
				enabled?: boolean
				return_value?: unknown
		  }
		| undefined = { enabled: false }
	export let moduleId: string = ''
	export let fullResult: boolean = false
	export let closeOnOutsideClick: boolean = false
	export let getLogs: boolean = false

	const dispatch = createEventDispatcher<{
		selectJob: Job
		updateMock: { enabled: boolean; return_value?: unknown }
	}>()

	let jsonView = false
	let clientHeight: number = 0
	let savedJsonData: any = {}
	let tmpMock: { enabled: boolean; return_value?: unknown } | undefined = undefined
	let error = ''
</script>

<div class="w-full h-full flex flex-col p-1" bind:clientHeight>
	<div class="flex flex-row items-center justify-between w-full">
		<div class="flex flex-row items-center gap-0.5">
			<Popover
				floatingConfig={{
					placement: 'left-start',
					offset: { mainAxis: 10, crossAxis: -6 },
					gutter: 0 // hack to make offset effective, see https://github.com/melt-ui/melt-ui/issues/528
				}}
				contentClasses="w-[275px]"
				{closeOnOutsideClick}
				disablePopup={isConnecting}
			>
				<svelte:fragment slot="trigger">
					<Button
						color="light"
						size="xs2"
						variant="contained"
						btnClasses="bg-transparent"
						startIcon={{ icon: History }}
						nonCaptureEvent
					/>
				</svelte:fragment>
				<svelte:fragment slot="content">
					<div class="rounded-[inherit]" style={`height: ${clientHeight}px`}>
						<StepHistory
							{moduleId}
							{getLogs}
							on:select={({ detail }) => {
								if (detail.result) {
									dispatch('selectJob', detail)
									savedJsonData = detail.result
									jsonData = detail.result
									//TODO: display warning approval here : this will override the mock value
									if (mock?.enabled) {
										const newMock = {
											enabled: true,
											return_value: detail.result ?? {}
										}
										dispatch('updateMock', newMock)
									}
								} else {
									jsonData = savedJsonData
								}
							}}
						/>
					</div>
				</svelte:fragment>
			</Popover>
			<ToggleSimple
				disabled={isConnecting}
				pressed={mock?.enabled ?? false}
				on:pressedChange={({ detail }) => {
					if (mock?.enabled && !detail) {
						const newMock = {
							enabled: false,
							return_value: mock?.return_value
						}
						dispatch('updateMock', newMock)
					} else if (detail && !!mock) {
						const newMock = {
							enabled: true,
							return_value: jsonData ?? { example: 'value' }
						}
						dispatch('updateMock', newMock)
					}
				}}
			>
				<Button
					color="light"
					size="xs2"
					variant="contained"
					btnClasses={`bg-transparent ${
						mock?.enabled
							? 'text-white bg-blue-500 hover:text-primary hover:bg-blue-700 hover:text-gray-100'
							: ''
					}`}
					startIcon={{ icon: Pin }}
					iconOnly
					nonCaptureEvent
				/>
			</ToggleSimple>

			{#if !jsonView}
				<Tooltip disablePopup={mock?.enabled}>
					<Button
						size="xs2"
						color="light"
						variant="contained"
						startIcon={{ icon: Pen }}
						on:click={() => {
							jsonView = true
						}}
						disabled={!mock?.enabled || isConnecting}
					/>
					<svelte:fragment slot="text">
						{'Enable mock to edit the output'}
					</svelte:fragment>
				</Tooltip>
			{:else}
				<Button
					size="xs2"
					color="green"
					variant="contained"
					startIcon={{ icon: Check }}
					on:click={() => {
						if (!tmpMock) {
							return
						}
						jsonView = false
						mock = tmpMock
						dispatch('updateMock', {
							enabled: tmpMock?.enabled ?? false,
							return_value: tmpMock?.return_value
						})
						jsonData = tmpMock?.return_value ?? {}
						tmpMock = undefined
					}}
					disabled={!!error || !tmpMock}
				/>
				<Button
					size="xs2"
					color="red"
					variant="contained"
					startIcon={{ icon: X }}
					on:click={() => {
						jsonView = false
					}}
				/>
			{/if}
		</div>
	</div>

	{#if fullResult}
		<slot />
	{:else}
		<div class="grow min-h-0 p-2 rounded-sm w-full overflow-auto">
			{#if !jsonData || jsonData === 'never tested this far'}
				<div class="flex flex-col items-center justify-center h-full">
					<p class="text-xs text-secondary">Test this step to see results</p>
				</div>
			{:else if jsonView}
				<JsonEditor
					bind:error
					small
					on:changeValue={({ detail }) => {
						if (mock?.enabled) {
							const newMock = {
								enabled: true,
								return_value: structuredClone(detail)
							}
							tmpMock = newMock
						}
					}}
					code={JSON.stringify(
						mock?.enabled && mock.return_value ? mock.return_value : jsonData,
						null,
						2
					)}
					class="h-full"
				/>
			{:else}
				<ObjectViewer
					json={mock?.enabled && mock.return_value && !isConnecting ? mock.return_value : jsonData}
					topBrackets={false}
					pureViewer={false}
					{prefix}
					on:select
					{allowCopy}
				/>
			{/if}
		</div>
	{/if}
</div>
