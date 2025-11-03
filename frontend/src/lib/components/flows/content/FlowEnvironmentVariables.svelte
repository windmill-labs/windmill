<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { getContext, setContext } from 'svelte'
	import type { PropPickerWrapperContext } from '../propPicker/PropPickerWrapper.svelte'
	import { writable } from 'svelte/store'
	import type { FlowEditorContext } from '../types'
	import { Button } from '$lib/components/common'
	import { Plus, Trash2 } from 'lucide-svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'

	interface Props {
		noEditor: boolean
	}

	type EnvVarType = 'string' | 'json'

	interface EnvVarEntry {
		id: string
		key: string
		value: any
		type: EnvVarType
		displayValue: string
		error?: string
	}

	let { noEditor }: Props = $props()

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	if (!flowStore.val.value.flow_env) {
		flowStore.val.value.flow_env = {}
	}
	let envVars = $derived(flowStore.val.value.flow_env || {})

	function determineValueType(value: any): EnvVarType {
		if (typeof value === 'string') {
			try {
				JSON.parse(value)
				return value.trim().startsWith('{') ||
					value.trim().startsWith('[') ||
					value.trim().startsWith('"')
					? 'json'
					: 'string'
			} catch {
				return 'string'
			}
		}
		return 'json'
	}

	let envTypes = $state<Record<string, EnvVarType>>({})

	$effect(() => {
		for (const [key, value] of Object.entries(envVars)) {
			if (!envTypes[key]) {
				envTypes[key] = determineValueType(value)
			}
		}
	})

	let envEntries = $derived(
		Object.entries(envVars).map(([key, value]): EnvVarEntry => {
			const stringValue = typeof value === 'string' ? value : JSON.stringify(value, null, 2)
			const type = envTypes[key] || determineValueType(value)
			return {
				id: key,
				key,
				value,
				type,
				displayValue: stringValue,
				error: undefined
			}
		})
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
		envTypes[newKey] = 'string'
		flowStore.val = flowStore.val
	}

	function removeEnvVar(key: string) {
		if (flowStore.val.value.flow_env && key in flowStore.val.value.flow_env) {
			delete flowStore.val.value.flow_env[key]
			delete envTypes[key]
			flowStore.val = flowStore.val
		}
	}

	function updateEnvValue(key: string, value: string, type: EnvVarType) {
		if (flowStore.val.value.flow_env) {
			if (type === 'json') {
				try {
					const parsed = JSON.parse(value)
					flowStore.val.value.flow_env[key] = parsed
				} catch (e) {
					flowStore.val.value.flow_env[key] = value
				}
			} else {
				flowStore.val.value.flow_env[key] = value
			}
			flowStore.val = flowStore.val
		}
	}

	function updateEnvKey(oldKey: string, newKey: string) {
		if (flowStore.val.value.flow_env && oldKey !== newKey && newKey.trim() !== '') {
			const value = flowStore.val.value.flow_env[oldKey]
			const type = envTypes[oldKey] || 'string'
			delete flowStore.val.value.flow_env[oldKey]
			delete envTypes[oldKey]
			flowStore.val.value.flow_env[newKey] = value
			envTypes[newKey] = type
			flowStore.val = flowStore.val
		}
	}

	function updateEnvType(key: string, newType: EnvVarType) {
		if (flowStore.val.value.flow_env && key in flowStore.val.value.flow_env) {
			const currentValue = flowStore.val.value.flow_env[key]
			const stringValue =
				typeof currentValue === 'string' ? currentValue : JSON.stringify(currentValue, null, 2)

			envTypes[key] = newType

			if (newType === 'json') {
				try {
					const parsed = JSON.parse(stringValue)
					flowStore.val.value.flow_env[key] = parsed
				} catch {
					flowStore.val.value.flow_env[key] = stringValue
				}
			} else {
				flowStore.val.value.flow_env[key] = stringValue
			}
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
				be used in JavaScript expressions and input bindings. You can choose between String or JSON
				types for each variable - JSON types allow complex data structures.
			</Alert>

			{#if envEntries.length === 0}
				<Alert type="warning" title="No environment variables" class="m-4">
					This flow has no environment variables defined. Click "Add Variable" to create your first
					environment variable.
				</Alert>
			{:else}
				<div class="space-y-4 p-4">
					{#each envEntries as entry (entry.id)}
						<div class="flex flex-col gap-4 p-4 border rounded-lg bg-surface-secondary">
							<div class="flex items-end gap-4">
								<label class="flex flex-col gap-1 text-sm font-medium min-w-0 flex-1">
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

								<label class="flex flex-col gap-1 text-sm font-medium w-32">
									<span>Type</span>
									<select
										value={entry.type}
										disabled={noEditor}
										class="input text-sm"
										onchange={(e) => {
											const newType = e.currentTarget.value as EnvVarType
											updateEnvType(entry.key, newType)
										}}
									>
										<option value="string">String</option>
										<option value="json">JSON</option>
									</select>
								</label>

								<div class="flex items-end">
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

							<div class="flex flex-col gap-1">
								<!-- svelte-ignore a11y_label_has_associated_control -->
								<label class="text-sm font-medium">Value</label>
								{#if entry.type === 'json'}
									<div class="w-full">
										<JsonEditor
											bind:code={entry.displayValue}
											disabled={noEditor}
											class="min-h-[60px] max-h-[200px]"
											on:change={() => {
												updateEnvValue(entry.key, entry.displayValue, 'json')
											}}
										/>
									</div>
								{:else}
									<input
										type="text"
										value={entry.displayValue}
										oninput={(e) => updateEnvValue(entry.key, e.currentTarget.value, 'string')}
										disabled={noEditor}
										class="input w-full"
										placeholder="Variable value"
									/>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</FlowCard>
</div>
