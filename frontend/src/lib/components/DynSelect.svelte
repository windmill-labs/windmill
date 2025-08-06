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
	import Tooltip from './Tooltip.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { type DynamicSelect } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { untrack } from 'svelte'

	interface Props {
		value?: any
		helperScript?: DynamicSelect.HelperScript
		entrypoint: string
		otherArgs?: Record<string, any>
		name: string
	}

	let { value = $bindable(), helperScript, entrypoint, otherArgs: otherArgs }: Props = $props()

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
								'Error in DynSelect function execution: ' +
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
			helperScript?.type == 'inline'
				? resultJobLoader?.runPreview(
						helperScript?.path ?? 'NO_PATH',
						helperScript.code,
						helperScript.lang,
						{ ...otherArgs, filterText, _ENTRYPOINT_OVERRIDE: entrypoint },
						undefined,
						undefined,
						undefined,
						cb
					)
				: resultJobLoader?.runScriptByHash(
						helperScript?.hash ?? 'NO_HASH',
						{ ...otherArgs, filterText, _ENTRYPOINT_OVERRIDE: entrypoint },
						cb
					)
		})
	}

	let neverLoaded = $state(true)

	$effect(() => {
		if (_items.value && value) {
			if (!_items.value.find((x) => x.value == value)) {
				value = undefined
			}
		}
	})

	let lastArgs = $state.snapshot(otherArgs)

	let timeout: NodeJS.Timeout | undefined = $state()
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
		<Select
			bind:value
			bind:open
			{items}
			bind:filterText
			loading={!open && _items.status === 'loading'}
			clearable
			noItemsMsg={_items.status === 'loading' ? 'Loading...' : 'No items found'}
		/>
		{#if _items.error}
			<div class="text-red-400 text-2xs">
				error: <Tooltip>{_items.error}</Tooltip>
			</div>
		{/if}
	</div>
{:else}
	<div class="flex flex-col gap-1 w-full">
		<div class="text-xs text-tertiary"
			>Dynamic Select is not available in this mode, write value directly</div
		>
		{#await import('$lib/components/JsonEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default code={JSON.stringify(value, null, 2)} bind:value />
		{/await}
	</div>
{/if}
