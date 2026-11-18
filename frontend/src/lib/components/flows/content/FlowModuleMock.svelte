<script lang="ts">
	import { run } from 'svelte/legacy'

	import Toggle from '$lib/components/Toggle.svelte'
	import type { FlowModule } from '$lib/gen'
	import Label from '$lib/components/Label.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import { untrack } from 'svelte'
	import { slideDynamic } from '$lib/transitions'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { nextMock } from '../flowMock'

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
		const newMock = nextMock(flowModule.mock, {
			...flowModule.mock,
			enabled: true,
			return_value: detail
		})

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
				flowModule.mock = { ...flowModule.mock, enabled: false }
			} else {
				flowModule.mock = nextMock(flowModule.mock, {
					...flowModule.mock,
					enabled: true,
					return_value: flowModule.mock?.return_value ?? { example: 'value' }
				})
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
		<div class="pl-9 flex flex-col gap-2" transition:slideDynamic>
			<Label label="Applies to">
				<ToggleButtonGroup
					selected={flowModule.mock?.preview_only ? 'preview' : 'every'}
					onSelected={(v) => {
						flowModule.mock = {
							...flowModule.mock,
							enabled: true,
							preview_only: v === 'preview'
						}
					}}
				>
					{#snippet children({ item })}
						<ToggleButton value="preview" label="Test runs" {item} />
						<ToggleButton value="every" label="Every run" {item} />
					{/snippet}
				</ToggleButtonGroup>
				<span class="text-2xs font-normal text-secondary">
					{flowModule.mock?.preview_only
						? 'Only runs started from the editor, and the subflows they call, use the pin.'
						: 'Deployed runs also return the pinned value.'}
				</span>
			</Label>
			<Label label="Pinned value">
				{#key renderCount}
					<JsonEditor {code} on:changeValue={updateMockValue} />
				{/key}
			</Label>
		</div>
	{/if}
</div>
