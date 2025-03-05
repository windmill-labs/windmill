<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Pin, History } from 'lucide-svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import ToggleSimple from '$lib/components/meltComponents/ToggleSimple.svelte'
	import { twMerge } from 'tailwind-merge'
	import StepHistory from './StepHistory.svelte'
	import { Popover } from '$lib/components/meltComponents'

	export let jsonData = {}
	export let selected: boolean = false
	export let prefix: string = ''
	export let allowCopy: boolean = false
	export let isConnecting: boolean = false
	export let pinned: boolean = false
	export let moduleId: string = ''

	let jsonView = false
	let clientHeight: number = 0
	let savedJsonData: any = {}
</script>

<div class="w-full h-full flex flex-col p-1" bind:clientHeight>
	{#if !isConnecting}
		<div class="flex flex-row items-center justify-between w-full">
			<div class="flex flex-row items-center gap-0.5">
				<Popover
					floatingConfig={{
						placement: 'left-start',
						offset: { mainAxis: 10, crossAxis: -6 },
						gutter: 0 // hack to make offset effective, see https://github.com/melt-ui/melt-ui/issues/528
					}}
					contentClasses={twMerge(
						selected
							? 'outline outline-offset-0  outline-2  outline-slate-500 dark:outline-gray-400'
							: '',
						'w-[275px]'
					)}
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
						<div style="height: {clientHeight}px">
							<StepHistory
								{moduleId}
								on:select={({ detail }) => {
									if (detail.result) {
										savedJsonData = detail.result
										jsonData = detail.result
									} else {
										jsonData = savedJsonData
									}
								}}
							/>
						</div>
					</svelte:fragment>
				</Popover>
				<ToggleSimple bind:pressed={pinned}>
					<Button
						color="light"
						size="xs2"
						variant="contained"
						btnClasses="bg-transparent"
						startIcon={{ icon: Pin }}
						iconOnly
						nonCaptureEvent
					/>
				</ToggleSimple>
			</div>
			<Toggle
				size="2xs"
				options={{
					right: 'JSON',
					rightTooltip:
						'Arguments can be edited either using the wizard, or by editing their JSON Schema.'
				}}
				textClass="text-2xs"
				bind:checked={jsonView}
			/>
		</div>
	{/if}
	<div class="grow min-h-0 p-2 rounded-sm w-full overflow-auto">
		{#if !jsonData || jsonData === 'never tested this far'}
			<div class="flex flex-col items-center justify-center h-full">
				<p class="text-xs text-secondary">Test this step to see results</p>
			</div>
		{:else if jsonView}
			<SimpleEditor
				small
				fixedOverflowWidgets={false}
				on:change={() => {
					jsonData = JSON.parse(JSON.stringify(jsonData))
				}}
				code={JSON.stringify(jsonData, null, 2)}
				lang="json"
				autoHeight
				automaticLayout
				class="h-full"
			/>
		{:else}
			<ObjectViewer
				json={jsonData}
				topBrackets={false}
				pureViewer={false}
				{prefix}
				on:select
				{allowCopy}
			/>
		{/if}
	</div>
</div>
