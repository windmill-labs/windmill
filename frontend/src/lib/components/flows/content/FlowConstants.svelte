<script lang="ts">
	import FlowCard from '../common/FlowCard.svelte'
	import { Alert, Button } from '$lib/components/common'
	import { getContext, setContext } from 'svelte'
	import type { PropPickerWrapperContext } from '../propPicker/PropPickerWrapper.svelte'
	import { writable } from 'svelte/store'
	import type { FlowEditorContext } from '../types'
	import { Trash, Plus } from 'lucide-svelte'

	interface Props {
		noEditor: boolean
	}

	let { noEditor }: Props = $props()

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	// Initialize env_variables if it doesn't exist
	if (!flowStore.val.value.env_variables) {
		flowStore.val.value.env_variables = {}
	}

	let envVariables = $derived(flowStore.val.value.env_variables ?? {})
	let envEntries = $derived(Object.entries(envVariables))

	function addEnvVariable() {
		if (!flowStore.val.value.env_variables) {
			flowStore.val.value.env_variables = {}
		}

		// Find a unique key name
		let counter = 1
		let newKey = `NEW_VAR`
		while (flowStore.val.value.env_variables[newKey]) {
			newKey = `NEW_VAR_${counter}`
			counter++
		}

		flowStore.val.value.env_variables[newKey] = ''
	}

	function removeEnvVariable(key: string) {
		if (flowStore.val.value.env_variables) {
			delete flowStore.val.value.env_variables[key]
		}
	}

	function updateEnvVariableKey(oldKey: string, newKey: string) {
		if (flowStore.val.value.env_variables && oldKey !== newKey) {
			const value = flowStore.val.value.env_variables[oldKey]
			delete flowStore.val.value.env_variables[oldKey]
			flowStore.val.value.env_variables[newKey] = value
		}
	}

	function updateEnvVariableValue(key: string, value: string) {
		if (flowStore.val.value.env_variables) {
			flowStore.val.value.env_variables[key] = value
		}
	}

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		inputMatches: writable(undefined),
		focusProp: () => {},
		propPickerConfig: writable(undefined),
		clearFocus: () => {}
	})
</script>

<div class="min-h-full">
	<FlowCard {noEditor} title="Environment Variables">
		{#snippet header()}
			<Button
				size="xs"
				color="blue"
				variant="border"
				startIcon={{ icon: Plus }}
				onclick={addEnvVariable}
			>
				Add Variable
			</Button>
		{/snippet}
		<div class="min-h-full flex-1">
			<Alert type="info" title="Environment Variables" class="m-4">
				Define environment variables that can be referenced throughout your flow using the
				<code class="text-xs">env</code> prefix (e.g., <code class="text-xs">env.FOO</code>).
				These variables are stored in the flow definition and can be easily updated when deploying
				or forking the flow.
			</Alert>

			{#if envEntries.length === 0}
				<Alert type="warning" title="No environment variables" class="m-4">
					This flow has no environment variables defined. Click "Add Variable" to create one.
				</Alert>
			{:else}
				<div class="p-4 space-y-3">
					{#each envEntries as [key, value], index (key + index)}
						<div class="flex items-center gap-2 p-3 border rounded bg-surface-secondary">
							<div class="flex-1 grid grid-cols-2 gap-2">
								<div>
									<label class="text-xs text-secondary mb-1 block">Variable Name</label>
									<input
										type="text"
										class="windmill-input"
										placeholder="VARIABLE_NAME"
										value={key}
										oninput={(e) => {
											const target = e.currentTarget as HTMLInputElement
											const newKey = target.value
											if (newKey && newKey !== key) {
												updateEnvVariableKey(key, newKey)
											}
										}}
									/>
								</div>
								<div>
									<label class="text-xs text-secondary mb-1 block">Value</label>
									<input
										type="text"
										class="windmill-input"
										placeholder="value"
										value={value}
										oninput={(e) => {
											const target = e.currentTarget as HTMLInputElement
											updateEnvVariableValue(key, target.value)
										}}
									/>
								</div>
							</div>
							<Button
								size="xs"
								color="red"
								variant="border"
								startIcon={{ icon: Trash }}
								iconOnly
								onclick={() => removeEnvVariable(key)}
								aria-label="Remove variable"
							/>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</FlowCard>
</div>
