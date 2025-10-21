<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { getContext, setContext } from 'svelte'
	import type { PropPickerWrapperContext } from '../propPicker/PropPickerWrapper.svelte'
	import { writable } from 'svelte/store'
	import type { FlowEditorContext } from '../types'
	import { Button } from '$lib/components/common'
	import { Plus, Trash2 } from 'lucide-svelte'
	import FlowCard from '../common/FlowCard.svelte'

	interface Props {
		noEditor: boolean
	}

	let { noEditor }: Props = $props()

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	if (!flowStore.val.value.flow_env) {
		flowStore.val.value.flow_env = {}
	}
	let envVars = $derived(flowStore.val.value.flow_env || {})
	let envEntries = $derived(
		Object.entries(envVars).map(([key, value]) => ({ id: key, key, value }))
	)

	function addEnvVar() {
		const existingKeys = Object.keys(envVars)
		let counter = 1
		let newKey = `VAR_${counter}`
		while (existingKeys.includes(newKey)) {
			counter++
			newKey = `VAR_${counter}`
		}

		if (!flowStore.val.value.flow_env) {
			flowStore.val.value.flow_env = {}
		}
		flowStore.val.value.flow_env[newKey] = ''
		flowStore.val = flowStore.val
	}

	function removeEnvVar(key: string) {
		if (flowStore.val.value.flow_env && key in flowStore.val.value.flow_env) {
			delete flowStore.val.value.flow_env[key]
			flowStore.val = flowStore.val
		}
	}

	function updateEnvValue(key: string, value: string) {
		if (flowStore.val.value.flow_env) {
			flowStore.val.value.flow_env[key] = value
			flowStore.val = flowStore.val
		}
	}

	function updateEnvKey(oldKey: string, newKey: string) {
		if (flowStore.val.value.flow_env && oldKey !== newKey && newKey.trim() !== '') {
			const value = flowStore.val.value.flow_env[oldKey]
			delete flowStore.val.value.flow_env[oldKey]
			flowStore.val.value.flow_env[newKey] = value
			flowStore.val = flowStore.val
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
			<Button size="xs" startIcon={{ icon: Plus }} onClick={addEnvVar} disabled={noEditor}>
				Add Variable
			</Button>
		{/snippet}
		<div class="min-h-full flex-1">
			<Alert type="info" title="Environment Variables" class="m-4">
				Environment variables can be referenced in any flow step using the syntax{' '}
				<code>env.VARIABLE_NAME</code>. These variables are available in the property picker and can
				be used in JavaScript expressions and input bindings.
			</Alert>

			{#if envEntries.length === 0}
				<Alert type="warning" title="No environment variables" class="m-4">
					This flow has no environment variables defined. Click "Add Variable" to create your first
					environment variable.
				</Alert>
			{:else}
				<div class="space-y-4 p-4">
					{#each envEntries as entry (entry.id)}
						<div class="flex items-end gap-4 p-4 border rounded-lg bg-surface-secondary">
							<label class="flex flex-col gap-1 text-sm font-medium">
								<span>Variable Name</span>
								<input
									type="text"
									value={entry.key}
									onblur={(e) => {
										const newKey = e.currentTarget.value.trim()
										if (newKey !== entry.key && newKey !== '') {
											updateEnvKey(entry.key, newKey)
										}
									}}
									disabled={noEditor}
									class="input"
									placeholder="VARIABLE_NAME"
								/>
							</label>

							<label class="flex flex-col gap-1 text-sm font-medium">
								<span>Value</span>
								<input
									type="text"
									value={entry.value}
									oninput={(e) => updateEnvValue(entry.key, e.currentTarget.value)}
									disabled={noEditor}
									class="input w-full"
									placeholder="Variable value"
								/>
							</label>
							<div class="flex">
								{#if !noEditor}
									<Button
										size="xs"
										color="red"
										startIcon={{ icon: Trash2 }}
										onClick={() => removeEnvVar(entry.key)}
									>
										Remove
									</Button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</FlowCard>
</div>
