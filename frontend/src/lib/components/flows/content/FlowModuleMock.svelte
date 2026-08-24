<script lang="ts">
	import { run } from 'svelte/legacy'

	import Toggle from '$lib/components/Toggle.svelte'
	import type { FlowModule } from '$lib/gen'
	import Label from '$lib/components/Label.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import { untrack } from 'svelte'
	import { slideDynamic } from '$lib/transitions'

	interface Props {
		flowModule: FlowModule
	}

	let { flowModule = $bindable() }: Props = $props()

	let code: string | undefined = $state(
		flowModule.mock?.return_value
			? JSON.stringify(flowModule.mock?.return_value, null, 2)
			: undefined
	)
	let isMockEnabled: boolean | undefined = $state(Boolean(flowModule.mock?.enabled))

	// Track the last value to prevent circular updates
	let lastMockValue = JSON.stringify(flowModule.mock)
	let renderCount = $state(0)

	function updateMock(
		newMock: { enabled?: boolean | undefined; return_value?: unknown } | undefined
	) {
		if (!newMock) return

		const newMockString = JSON.stringify(newMock)

		// Only update if it's actually a new value
		if (newMockString !== lastMockValue) {
			renderCount++
			lastMockValue = newMockString
			code = newMock.return_value ? JSON.stringify(newMock.return_value, null, 2) : undefined
			isMockEnabled = Boolean(newMock?.enabled)
		}
	}
	run(() => {
		;(flowModule.mock, untrack(() => updateMock(flowModule.mock)))
	})

	function updateMockValue({ detail }: any) {
		const newMock = {
			enabled: true,
			return_value: detail
		}

		// Update the last value to prevent circular updates
		lastMockValue = JSON.stringify(newMock)
		flowModule.mock = newMock
	}
</script>

<div class="flex flex-col gap-2">
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={isMockEnabled}
		on:change={() => {
			if (isMockEnabled) {
				flowModule.mock = {
					enabled: false,
					return_value: flowModule.mock?.return_value
				}
			} else {
				flowModule.mock = {
					enabled: true,
					return_value: flowModule.mock?.return_value ?? { example: 'value' }
				}
				code = JSON.stringify(flowModule.mock?.return_value, null, 2)
			}
		}}
		options={{
			right: 'Pin output',
			rightTooltip:
				'While pinned, the step returns this value immediately instead of executing. The same control lives on the step in the graph.'
		}}
	/>
	{#if isMockEnabled}
		<div class="pl-9" transition:slideDynamic>
			<Label label="Pinned value">
				{#key renderCount}
					<JsonEditor {code} on:changeValue={updateMockValue} />
				{/key}
			</Label>
		</div>
	{/if}
</div>
