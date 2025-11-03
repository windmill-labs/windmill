<script lang="ts" module>
	function validSelectObject(x): string | undefined {
		if (typeof x != 'object') {
			return JSON.stringify(x) + ' is not an object'
		}
		let keys = Object.keys(x)
		if (!keys.includes('value') || !keys.includes('label')) {
			return JSON.stringify(x) + ' does not contain value or label field'
		}
		if (typeof x['label'] != 'string') {
			return JSON.stringify(x) + ' label is not a string'
		}
		return
	}
</script>

<script lang="ts">
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import JobLoader, { type Callbacks } from './JobLoader.svelte'
	import Select from './select/Select.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { type DynamicInput } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { untrack } from 'svelte'

	interface Props {
		value?: any
		helperScript?: DynamicInput.HelperScript
		format: string
		otherArgs?: Record<string, any>
		name: string
	}

	let { value = $bindable(), helperScript, format, otherArgs: otherArgs }: Props = $props()

	let [inputType, entrypoint] = $derived(format.includes('-') ? format.split('-', 2) : [format, ''])

	let isMultiple = $derived(inputType === 'dynmultiselect')
	let isSelect = $derived(inputType === 'dynselect' || inputType === 'dynmultiselect')

	$effect.pre(() => {
		if (isMultiple && value === undefined) {
			value = []
		}
	})

	let resultJobLoader: JobLoader | undefined = $state()
	let _items = usePromise(getItemsFromOptions, { clearValueOnRefresh: false })
	let items = $derived(_items.value)

	let filterText: string = $state('')
	let open: boolean = $state(false)

	async function getItemsFromOptions() {
		return new Promise<{ label: string; value: any }[]>((resolve, reject) => {
			let cb: Callbacks = {
				doneResult({ result }) {
					if (!result || !Array.isArray(result)) {
						if (result?.error?.message && result?.error?.name) {
							reject(
								`Error in ${inputType} function execution: ` +
									result?.error?.name +
									' - ' +
									result?.error?.message
							)
						} else {
							reject('Result was not an array but ' + JSON.stringify(result, null, 2))
						}
						return
					}
					if (result.length == 0) resolve([])

					if (result.every((x) => typeof x == 'string')) {
						result = result.map((x) => ({ label: x, value: x }))
					} else if (result.find((x) => validSelectObject(x) != undefined)) {
						reject(validSelectObject(result.find((x) => validSelectObject(x) != undefined)))
						return
					}
					resolve(result)
				},
				cancel: () => reject(),
				doneError({ id, error }) {
					reject(error)
				}
			}
			resultJobLoader?.runDynamicInputScript(
				entrypoint,
				helperScript!,
				{ ...otherArgs, filterText, _ENTRYPOINT_OVERRIDE: entrypoint },
				cb
			)
		})
	}

	let neverLoaded = $state(true)

	$effect(() => {
		if (_items.value && value !== undefined && isSelect) {
			if (isMultiple && Array.isArray(value) && Array.isArray(_items.value)) {
				const availableValues = new Set(_items.value.map((x) => x.value))
				const filteredValue = value.filter((v) => availableValues.has(v))
				if (filteredValue.length !== value.length) {
					value = filteredValue
				}
			} else if (!isMultiple && value !== undefined) {
				if (!_items.value.find((x) => x.value == value)) {
					value = undefined
				}
			}
		}
	})

	let lastArgs = $state.snapshot(otherArgs)

	let timeout: number | undefined = $state()
	let nargs = $state($state.snapshot(otherArgs))
	$effect(() => {
		otherArgs
		untrack(() => clearTimeout(timeout))
		timeout = setTimeout(() => {
			nargs = $state.snapshot(otherArgs)
		}, 1000)
	})

	$effect(() => {
		;[filterText, entrypoint, helperScript]
		if (resultJobLoader && (open || neverLoaded || !deepEqual(lastArgs, nargs))) {
			neverLoaded = false
			lastArgs = $state.snapshot(otherArgs)
			_items.refresh()
		}
	})
</script>

{#if helperScript}
	<JobLoader onlyResult bind:this={resultJobLoader} />

	<div class="w-full flex-col flex">
		{#if inputType === 'dynmultiselect'}
			<MultiSelect
				bind:value
				items={safeSelectItems(items || [])}
				placeholder="Select items"
				noItemsMsg={_items.status === 'loading' ? 'Loading...' : 'No items found'}
				disabled={_items.status === 'loading'}
			/>
		{:else if inputType === 'dynselect'}
			<Select
				bind:value
				bind:open
				{items}
				bind:filterText
				loading={!open && _items.status === 'loading'}
				clearable
				noItemsMsg={_items.status === 'loading' ? 'Loading...' : 'No items found'}
			/>
		{:else}
			<!-- Future dynamic input types can be added here -->
			<div class="text-red-400 text-sm">
				Unsupported dynamic input type: {inputType}
			</div>
		{/if}
		{#if _items.error}
			<div class="text-red-400 text-2xs">
				error: <Tooltip>{_items.error}</Tooltip>
			</div>
		{/if}
	</div>
{:else}
	<div class="flex flex-col gap-1 w-full">
		<div class="text-xs text-primary"
			>Dynamic input ({inputType}) is not available in this mode, write value directly</div
		>
		{#await import('$lib/components/JsonEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default code={JSON.stringify(value, null, 2)} bind:value />
		{/await}
	</div>
{/if}
