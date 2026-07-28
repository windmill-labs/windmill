<script lang="ts">
	import { getContext, setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import PropPicker from '$lib/components/propertyPicker/PropPicker.svelte'
	import FlowPlugConnect from '$lib/components/FlowPlugConnect.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import type { PickableProperties } from '../previousResults'
	import type { PropPickerWrapperContext } from './PropPickerWrapper.svelte'
	import { useConnect } from './useConnect.svelte'

	interface Props {
		/** Identifies this input within the panel; arming another one disarms it. */
		id: string
		pickableProperties: PickableProperties | undefined
		result?: any
		extraResults?: any
		/** Receives the picked property, whether it came from the popover or the graph. */
		onSelect: (path: string) => void
		disabled?: boolean
	}

	let { id, pickableProperties, result, extraResults, onSelect, disabled = false }: Props = $props()

	const propPickerContext = getContext<PropPickerContext | undefined>('PropPickerContext')

	const inModalPanel = $derived(propPickerContext?.inModalPanel?.() ?? false)

	const connect = useConnect({
		inModalPanel: () => inModalPanel,
		hasPickableProperties: () => pickableProperties != undefined,
		flowPropPickerConfig: propPickerContext?.flowPropPickerConfig ?? writable(undefined)
	})

	// PropPicker reads these to filter and highlight against what is being typed. Only the
	// step input form produces that signal, so here they stay empty.
	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		propPickerConfig: writable(undefined),
		inputMatches: writable(undefined),
		exprBeingEdited: writable([]),
		connectProp: () => {},
		clearConnect: () => connect.disarm(),
		pickerMode: () => 'popover' as const,
		pickableProperties: () => undefined
	})

	let open = $state(false)

	// A pick from the graph resolves without touching the popover, so follow the armed
	// slot rather than tracking open/closed separately.
	$effect(() => {
		if (!connect.isArmed(id)) {
			open = false
		}
	})
</script>

<Popover
	bind:isOpen={open}
	class="flex"
	placement="bottom-start"
	closeOnOutsideClick
	contentClasses="rounded-md border bg-surface shadow-lg overflow-hidden"
	on:openChange={({ detail }) => (detail ? connect.arm({ id, onSelect }) : connect.disarm())}
>
	{#snippet trigger()}
		<FlowPlugConnect
			connecting={connect.isArmed(id)}
			disableAnimation={inModalPanel}
			{disabled}
			title="Connect a property from a previous step"
		/>
	{/snippet}
	{#snippet content()}
		<div class="max-h-80 w-72 overflow-auto p-2">
			{#if pickableProperties}
				<PropPicker
					{pickableProperties}
					previousId={pickableProperties?.previousId}
					{result}
					{extraResults}
					on:select={({ detail }) => {
						connect.resolve(detail)
						open = false
					}}
				/>
			{:else}
				<div class="p-2 text-xs text-tertiary">Nothing to pick from yet.</div>
			{/if}
		</div>
	{/snippet}
</Popover>
