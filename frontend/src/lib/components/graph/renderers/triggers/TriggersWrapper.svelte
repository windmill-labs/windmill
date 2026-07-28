<script lang="ts">
	import { useOverlayStack } from '$lib/components/flows/overlayStack.svelte'
	import { openedDrawers } from '$lib/components/common/drawer/Disposable.svelte'
	import { NODE, type FlowNodeColorClasses } from '../../util'
	import { createEventDispatcher } from 'svelte'
	import type { TriggerType } from '$lib/components/triggers/utils'
	import TriggersBadge from './TriggersBadge.svelte'
	import { Plus } from 'lucide-svelte'
	import InsertModuleInner from '$lib/components/flows/map/InsertModuleInner.svelte'
	import AddTriggersButton from '$lib/components/triggers/AddTriggersButton.svelte'
	import { twMerge } from 'tailwind-merge'
	import Portal from '$lib/components/Portal.svelte'
	import { flip, offset, shift } from 'svelte-floating-ui/dom'
	import { createFloatingActions, type ComputeConfig } from 'svelte-floating-ui'
	import { clickOutside } from '$lib/utils'

	interface Props {
		path: string
		newItem: boolean
		selected: boolean
		isEditor?: boolean
		disableAi?: boolean
		showDraft?: boolean
		colorClasses: FlowNodeColorClasses
		onSelect?: (triggerIndex: number) => void
		onAddDraftTrigger?: (type: TriggerType) => void
	}

	let {
		path,
		newItem,
		selected,
		isEditor = false,
		disableAi = false,
		showDraft,
		onSelect,
		colorClasses,
		onAddDraftTrigger
	}: Props = $props()

	let showTriggerScriptPicker = $state(false)

	const pickerOverlay = useOverlayStack(() => showTriggerScriptPicker, openedDrawers)
	let numberOfTriggers = $state(0)

	const dispatch = createEventDispatcher()

	let floatingConfig: ComputeConfig = {
		strategy: 'fixed',
		// @ts-ignore
		placement: 'bottom',
		middleware: [offset(8), flip(), shift({ padding: 8 })],
		autoUpdate: true
	}

	const [floatingRef, floatingContent] = createFloatingActions(floatingConfig)
</script>

<div style={`width: ${NODE.width}px;`} use:floatingRef>
	<button
		class="relative flex w-full flex-row gap-1.5 px-2 p-1 items-center justify-center rounded-md drop-shadow-sm {colorClasses.outline} {colorClasses.bg}"
		style="height: {NODE.height}px"
		onclick={() => dispatch('select')}
	>
		<div
			class={twMerge(
				'flex flex-row items-center text-2xs font-normal',
				colorClasses.text,
				numberOfTriggers > 6 ? 'absolute left-0 -top-[20px]' : ''
			)}
		>
			Triggers
		</div>

		<TriggersBadge
			showOnlyWithCount={false}
			{showDraft}
			{path}
			{newItem}
			isFlow
			{selected}
			bind:numberOfTriggers
			limit={isEditor ? 7 : 8}
			{onSelect}
		/>

		{#if isEditor}
			<AddTriggersButton
				onAddScheduledPoll={() => {
					showTriggerScriptPicker = true
				}}
				class="w-fit h-fit"
				isEditor
				{onAddDraftTrigger}
			>
				<button
					class="hover:bg-slate-300 dark:hover:bg-slate-600 rounded-md outline-1 outline-dashed outline-secondary outline-offset-[-1px] text-xs w-[23px] h-[23px] relative center-center cursor-pointer text-secondary"
				>
					<Plus size={12} />
				</button>
			</AddTriggersButton>
		{/if}
	</button>
</div>

<svelte:window
	onkeydown={(e) => {
		if (showTriggerScriptPicker && e.key === 'Escape') {
			if (!pickerOverlay.isTopmost()) return
			showTriggerScriptPicker = false
		}
	}}
/>

{#if showTriggerScriptPicker}
	<Portal target="#flow-editor">
		<!-- Not a Popover: this opens from a menu item inside AddTriggersButton, which
		     closes before the picker renders, so there is no trigger element to bind to.
		     Same box and dismiss behaviour as the other module pickers. -->
		<div
			class="border rounded-lg shadow-lg bg-surface z5000 p-2 h-[400px]"
			use:floatingContent
			use:clickOutside={{ onClickOutside: () => (showTriggerScriptPicker = false) }}
		>
			<InsertModuleInner
				small
				{disableAi}
				on:new={(e) => {
					showTriggerScriptPicker = false
					dispatch('new', e.detail)
				}}
				on:pickScript={(e) => {
					showTriggerScriptPicker = false
					dispatch('pickScript', e.detail)
				}}
				kind="trigger"
			/>
		</div>
	</Portal>
{/if}
