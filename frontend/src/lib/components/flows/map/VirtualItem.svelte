<script lang="ts">
	import { Badge } from '$lib/components/common'

	import VirtualItemWrapper from './VirtualItemWrapper.svelte'
	import OutputPicker from '$lib/components/flows/propPicker/OutputPicker.svelte'
	import OutputPickerInner from '$lib/components/flows/propPicker/OutputPickerInner.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { fade } from 'svelte/transition'
	import { Database, Square } from 'lucide-svelte'
	import { useSvelteFlow } from '@xyflow/svelte'

	export let label: string | undefined = undefined
	export let bgColor: string = ''
	export let bgHoverColor: string = ''
	export let selected: boolean
	export let selectable: boolean
	export let id: string | undefined = undefined
	export let center = true
	export let borderColor: string | undefined = undefined
	export let hideId: boolean = false
	export let preLabel: string | undefined = undefined
	export let inputJson: Object | undefined = undefined
	export let prefix = ''
	export let alwaysPluggable: boolean = false
	export let cache: boolean = false
	export let earlyStop: boolean = false
	export let editMode: boolean = false

	const { viewport } = useSvelteFlow()
</script>

<VirtualItemWrapper
	{label}
	{bgColor}
	{bgHoverColor}
	{selected}
	{selectable}
	{id}
	on:select
	let:hover
>
	<div class="flex flex-col w-full">
		<div
			style={borderColor ? `border-color: ${borderColor};` : 'border: 0'}
			class="flex flex-row gap-1 justify-between {center
				? 'items-center'
				: 'items-baseline'} w-full overflow-hidden rounded-sm border p-2 text-2xs module text-primary border-gray-400 dark:border-gray-600"
		>
			{#if $$slots.icon}
				<slot name="icon" />
				<span class="mr-2"></span>
			{/if}
			<div class="flex flex-col flex-grow shrink-0 max-w-full min-w-0">
				{#if label}
					<div class="truncate text-center">{label}</div>
				{/if}
				{#if preLabel}
					<div class="truncate text-2xs text-center"><pre>{preLabel}</pre></div>
				{/if}
			</div>
			{#if id && !hideId && !id?.startsWith('subflow:')}
				<div class="flex items-center shrink min-w-0">
					<Badge color="indigo" wrapperClass="w-full" baseClass="max-w-full" title={id}>
						<span class="max-w-full text-2xs truncate">{id}</span>
					</Badge>
				</div>
			{/if}
		</div>
		{#if (alwaysPluggable || (inputJson && Object.keys(inputJson).length > 0)) && editMode}
			<OutputPicker
				zoom={$viewport?.zoom ?? 1}
				{selected}
				{hover}
				let:allowCopy
				isConnectingCandidate={true}
				let:isConnecting
				let:selectConnection
				variant="virtual"
			>
				<OutputPickerInner
					{allowCopy}
					{prefix}
					connectingData={isConnecting ? inputJson : undefined}
					on:select={selectConnection}
					moduleId={''}
					on:updateMock
					hideHeaderBar
					simpleViewer={inputJson}
					rightMargin
					historyOffset={{ mainAxis: 12, crossAxis: -9 }}
					class="p-1"
				/>
			</OutputPicker>
		{/if}
	</div>
	<div class="absolute text-sm right-12 -bottom-3 flex flex-row gap-1 z-10">
		{#if cache}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center rounded border bg-surface border-gray-400 text-secondary px-1 py-0.5"
				>
					<Database size={12} />
				</div>
				<svelte:fragment slot="text">Cached</svelte:fragment>
			</Popover>
		{/if}
		{#if earlyStop}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
				>
					<Square size={12} />
				</div>
				<svelte:fragment slot="text">Early stop if condition met</svelte:fragment>
			</Popover>
		{/if}
	</div>
</VirtualItemWrapper>
