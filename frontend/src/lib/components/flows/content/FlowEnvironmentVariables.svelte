<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { getContext, setContext } from 'svelte'
	import type { PropPickerWrapperContext } from '../propPicker/PropPickerWrapper.svelte'
	import { writable } from 'svelte/store'
	import type { FlowEditorContext } from '../types'
	import { Button } from '$lib/components/common'
	import { DollarSign, Plus, Trash2 } from 'lucide-svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		noEditor: boolean
	}

	type EnvVarType = 'string' | 'json' | 'resource'

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
			if (value.startsWith('$res:')) {
				return 'resource'
			}
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

	const typeOptions: { label: string; value: EnvVarType }[] = [
		{ label: 'String', value: 'string' },
		{ label: 'JSON', value: 'json' },
		{ label: 'Resource', value: 'resource' }
	]

	// Track resource paths separately for bind:value with ResourcePicker
	let resourcePaths = $state<Record<string, string | undefined>>({})

	// Initialize resourcePaths from existing flow_env values
	for (const [key, value] of Object.entries(flowStore.val.value.flow_env || {})) {
		if (typeof value === 'string' && value.startsWith('$res:')) {
			resourcePaths[key] = value.substring('$res:'.length)
		}
	}

	// Initialize types for new keys and sync resourcePaths → flow_env
	$effect(() => {
		for (const [key, value] of flowEnvVarsMap.entries()) {
			if (!flowEnvTypes[key]) {
				flowEnvTypes[key] = determineValueType(value)
			}
		}
		for (const [key, path] of Object.entries(resourcePaths)) {
			if (flowStore.val.value.flow_env && flowEnvTypes[key] === 'resource') {
				const newVal = '$res:' + (path || '')
				if (flowStore.val.value.flow_env[key] !== newVal) {
					flowStore.val.value.flow_env[key] = newVal
					flowStore.val = flowStore.val
				}
			}
		}
	})

	// Convert values when user changes the type dropdown
	let prevTypes: Record<string, EnvVarType> = {}
	$effect(() => {
		for (const [key, type] of Object.entries(flowEnvTypes)) {
			if (prevTypes[key] && prevTypes[key] !== type) {
				updateEnvType(key, type)
			}
			prevTypes[key] = type
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
			delete resourcePaths[key]
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

			// Move resource path if applicable
			if (type === 'resource' && oldKey in resourcePaths) {
				resourcePaths[newKey] = resourcePaths[oldKey]
				delete resourcePaths[oldKey]
			}

			flowStore.val = flowStore.val
		}
	}

	function updateEnvType(key: string, newType: EnvVarType) {
		if (flowStore.val.value.flow_env && key in flowStore.val.value.flow_env) {
			flowEnvTypes[key] = newType

			if (newType === 'resource') {
				flowStore.val.value.flow_env[key] = '$res:'
				resourcePaths[key] = ''
			} else if (newType === 'json') {
				delete resourcePaths[key]
				const currentValue = flowStore.val.value.flow_env[key]
				if (typeof currentValue === 'string') {
					try {
						flowStore.val.value.flow_env[key] = JSON.parse(currentValue)
					} catch {
						// keep as string if not valid JSON
					}
				}
			} else {
				delete resourcePaths[key]
				const currentValue = flowStore.val.value.flow_env[key]
				if (typeof currentValue !== 'string') {
					flowStore.val.value.flow_env[key] = JSON.stringify(currentValue, null, 2)
				}
			}
			flowStore.val = flowStore.val
		}
	}

	function setVarPath(key: string, path: string) {
		if (flowStore.val.value.flow_env) {
			flowStore.val.value.flow_env[key] = '$var:' + path
			flowStore.val = flowStore.val
		}
	}

	let variablePicker: ItemPicker | undefined = $state(undefined)
	let pickForKey: string | undefined = $state(undefined)

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
				input bindings. String values can link to workspace variables using the <DollarSign size={12}
					class="inline" /> button. Resource type references workspace resources resolved at runtime.
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

							<Label label="Value">
								{#if entry.type === 'resource'}
									<ResourcePicker
										bind:value={resourcePaths[entry.key]}
										disabled={noEditor}
									/>
								{:else if entry.type === 'json'}
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
									<div class="relative group w-full">
										<input
											type="text"
											value={entry.displayValue}
											oninput={(e) =>
												updateEnvValue(entry.key, e.currentTarget.value, 'string')}
											disabled={noEditor}
											class="input w-full"
											placeholder="Variable value"
										/>
										{#if !noEditor}
											<Button
												iconOnly
												startIcon={{ icon: DollarSign }}
												unifiedSize="sm"
												onClick={() => {
													pickForKey = entry.key
													variablePicker?.openDrawer?.()
												}}
												wrapperClasses="opacity-0 group-hover:opacity-100 transition-opacity absolute right-2 top-1/2 -translate-y-1/2 bg-surface-input"
												variant="subtle"
												title="Insert a Variable"
											/>
										{/if}
									</div>
									{#if typeof entry.value === 'string' && entry.value.startsWith('$var:') && entry.value.length > 5}
										<div class="text-2xs text-tertiary">
											Linked to variable <a
												href="/variables#{entry.value.slice(5)}"
												target="_blank"
												class="text-accent underline font-normal"
												>{entry.value.slice(5)}</a
											>
										</div>
									{/if}
								{/if}
							</Label>
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

<ItemPicker
	bind:this={variablePicker}
	pickCallback={(path, _) => {
		if (pickForKey) {
			setVarPath(pickForKey, path)
			pickForKey = undefined
		}
	}}
	itemName="Variable"
	extraField="path"
	loadItems={async () =>
		(await VariableService.listVariable({ workspace: $workspaceStore ?? '' })).map((x) => ({
			name: x.path,
			...x
		}))}
/>
