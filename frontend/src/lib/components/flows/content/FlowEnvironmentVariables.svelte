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
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'

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
	let flowEnvVarsMap = $derived(new Map(Object.entries(flowStore.val.value.flow_env || {})))

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

	let flowEnvTypes = $state<Record<string, EnvVarType>>({})

	const typeOptions = [
		{ label: 'String', value: 'string' as EnvVarType },
		{ label: 'JSON', value: 'json' as EnvVarType }
	]

	$effect(() => {
		for (const [key, value] of flowEnvVarsMap.entries()) {
			if (!flowEnvTypes[key]) {
				flowEnvTypes[key] = determineValueType(value)
			}
		}
	})

	$effect(() => {
		for (const [key, type] of Object.entries(flowEnvTypes)) {
			if (flowStore.val.value.flow_env && key in flowStore.val.value.flow_env) {
				const currentType = determineValueType(flowStore.val.value.flow_env[key])
				if (currentType !== type) {
					updateEnvType(key, type)
				}
			}
		}
	})

	let flowEnvEntries = $derived(
		Array.from(flowEnvVarsMap.entries()).map(([key, value]): EnvVarEntry => {
			const stringValue = typeof value === 'string' ? value : JSON.stringify(value, null, 2)
			const type = flowEnvTypes[key] || determineValueType(value)
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
		const existingKeys = Array.from(flowEnvVarsMap.keys())
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
		flowEnvTypes[newKey] = 'string'
		flowStore.val = flowStore.val
	}

	function removeEnvVar(key: string) {
		if (flowStore.val.value.flow_env && key in flowStore.val.value.flow_env) {
			delete flowStore.val.value.flow_env[key]
			delete flowEnvTypes[key]
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
			const type = flowEnvTypes[oldKey] || 'string'

			const newEnvVars: Record<string, any> = {}
			for (const [k, v] of flowEnvVarsMap.entries()) {
				if (k === oldKey) {
					newEnvVars[newKey] = value
				} else {
					newEnvVars[k] = v
				}
			}

			flowStore.val.value.flow_env = newEnvVars
			delete flowEnvTypes[oldKey]
			flowEnvTypes[newKey] = type
			flowStore.val = flowStore.val
		}
	}

	function updateEnvType(key: string, newType: EnvVarType) {
		if (flowStore.val.value.flow_env && key in flowStore.val.value.flow_env) {
			const currentValue = flowStore.val.value.flow_env[key]
			const stringValue =
				typeof currentValue === 'string' ? currentValue : JSON.stringify(currentValue, null, 2)

			flowEnvTypes[key] = newType

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
		connectProp: () => {},
		propPickerConfig: writable(undefined),
		clearConnect: () => {},
		exprBeingEdited: writable([])
	})
</script>

<div class="min-h-full">
	<FlowCard {noEditor} title="Flow Env Variables">
		<div class="min-h-full flex-1">
			<Alert type="info" title="Flow Env Variables" class="m-4">
				Flow envs can be referenced in any flow step input using the syntax{' '}
				<code>flow_env.VARIABLE_NAME</code> or <code>flow_env["VARIABLE_NAME"]</code>. These
				variables are available in the property picker and can be used in JavaScript expressions and
				input bindings. You can choose between String or JSON types for each variable - JSON types
				allow complex data structures.
			</Alert>

			{#if flowEnvEntries.length === 0}
				<Alert type="warning" title="No flow env variables" class="m-4">
					This flow has no flow env variables defined. Click "Add Variable" to create your first
					flow env variable.
				</Alert>
			{:else}
				<div class="space-y-4 p-4">
					{#each flowEnvEntries as entry (entry.id)}
						<div class="flex flex-col gap-4 p-4 border rounded-lg bg-surface-secondary">
							<div class="flex items-end gap-3">
								<div class="flex-1 min-w-0 max-w-xs">
									<Label label="Variable Name">
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
											class="input w-full"
											placeholder="VARIABLE_NAME"
										/>
									</Label>
								</div>

								<Label label="Type">
									<Select
										bind:value={flowEnvTypes[entry.key]}
										items={typeOptions}
										disabled={noEditor}
										size="sm"
										class="text-sm"
									/>
								</Label>

								{#if !noEditor}
									<Button
										size="sm"
										color="red"
										startIcon={{ icon: Trash2 }}
										onClick={() => removeEnvVar(entry.key)}
									>
										Remove
									</Button>
								{/if}
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
			<div class=" p-4">
				{#if !noEditor}
					<Button size="sm" startIcon={{ icon: Plus }} onClick={addEnvVar} color="light">
						Add Variable
					</Button>
				{/if}
			</div>
		</div>
	</FlowCard>
</div>
