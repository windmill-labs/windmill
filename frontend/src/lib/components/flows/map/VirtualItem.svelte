<script lang="ts">
	import { Badge } from '$lib/components/common'

	import VirtualItemWrapper from './VirtualItemWrapper.svelte'
	import OutputPicker from '$lib/components/flows/propPicker/OutputPicker.svelte'
	import OutputPickerInner from '$lib/components/flows/propPicker/OutputPickerInner.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { fade } from 'svelte/transition'
	import { Database, Square } from 'lucide-svelte'

	interface Props {
		label?: string | undefined
		bgColor?: string
		bgHoverColor?: string
		selected: boolean
		selectable: boolean
		id?: string | undefined
		center?: boolean
		borderColor?: string | undefined
		hideId?: boolean
		preLabel?: string | undefined
		inputJson?: Object | undefined
		prefix?: string
		alwaysPluggable?: boolean
		cache?: boolean
		earlyStop?: boolean
		editMode?: boolean
		icon?: import('svelte').Snippet
		onUpdateMock?: (mock: { enabled: boolean; return_value?: unknown }) => void
		onEditInput?: (moduleId: string, key: string) => void
	}

	let {
		label = undefined,
		bgColor = '',
		bgHoverColor = '',
		selected,
		selectable,
		id = undefined,
		center = true,
		borderColor = undefined,
		hideId = false,
		preLabel = undefined,
		inputJson = undefined,
		prefix = '',
		alwaysPluggable = false,
		cache = false,
		earlyStop = false,
		editMode = false,
		icon,
		onUpdateMock,
		onEditInput
	}: Props = $props()

	const outputPickerVisible = $derived(
		(alwaysPluggable || (inputJson && Object.keys(inputJson).length > 0)) && editMode
	)
</script>

<VirtualItemWrapper
	{label}
	{bgColor}
	{bgHoverColor}
	{selected}
	{selectable}
	{id}
	outputPickerVisible={outputPickerVisible ?? false}
	on:select
>
	{#snippet children({ hover })}
		<div class="flex flex-col w-full">
			<div
				style={borderColor ? `border-color: ${borderColor};` : 'border: 0'}
				class="flex flex-row gap-1 justify-between {center
					? 'items-center'
					: 'items-baseline'} w-full overflow-hidden rounded-sm border p-2 text-2xs module text-primary border-gray-400 dark:border-gray-600"
			>
				{#if icon}
					{@render icon?.()}
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
			{#if outputPickerVisible}
				<OutputPicker
					{selected}
					{hover}
					id={id ?? ''}
					isConnectingCandidate={true}
					variant="virtual"
				>
					{#snippet children({ allowCopy, isConnecting, selectConnection })}
						<OutputPickerInner
							{allowCopy}
							{prefix}
							connectingData={isConnecting ? inputJson : undefined}
							onSelect={selectConnection}
							moduleId={''}
							{onUpdateMock}
							hideHeaderBar
							simpleViewer={inputJson}
							rightMargin
							historyOffset={{ mainAxis: 12, crossAxis: -9 }}
							clazz="p-1"
							{onEditInput}
							selectionId={id ?? label ?? ''}
						/>
					{/snippet}
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
					{#snippet text()}
						Cached
					{/snippet}
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
					{#snippet text()}
						Early stop if condition met
					{/snippet}
				</Popover>
			{/if}
		</div>
	{/snippet}
</VirtualItemWrapper>
