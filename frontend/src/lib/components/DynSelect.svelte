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
	import type { Script } from '$lib/gen'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { deepEqual } from 'fast-equals'
	import JobLoader from './JobLoader.svelte'
	import Select from './select/Select.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { readFieldsRecursively } from '$lib/utils'

	interface Props {
		value?: any
		helperScript?:
			| { type: 'inline'; path?: string; lang: Script['language']; code: string }
			| { type: 'hash'; hash: string }
		entrypoint: string
		args?: Record<string, any>
		name: string
	}

	let { value = $bindable(), helperScript, entrypoint, args: _args, name }: Props = $props()

	let args = $state(structuredClone($state.snapshot(_args)))
	$effect(() => {
		readFieldsRecursively(_args, { excludeField: [name] })
		untrack(() => !deepEqual(args, _args) && (args = $state.snapshot(_args)))
	})

	async function getItemsFromOptions() {
		return new Promise<{ label: string; value: any }[]>((resolve, reject) => {
			let cb = {
				done(res) {
					if (!res || !Array.isArray(res)) {
						reject('Result was not an array')
						return
					}
					if (res.length == 0) resolve([])
					if (res.every((x) => typeof x == 'string')) {
						res = res.map((x) => ({ label: x, value: x }))
					} else if (res.find((x) => validSelectObject(x) != undefined)) {
						reject(validSelectObject(res.find((x) => validSelectObject(x) != undefined)))
					} else {
						if (filterText != undefined && filterText != '')
							res = res.filter((x) => x['label'].includes(filterText))
						resolve(res)
					}
				},
				cancel: () => reject(),
				error: (err) => reject(err)
			}
			helperScript?.type == 'inline'
				? resultJobLoader?.runPreview(
						helperScript?.path ?? 'NO_PATH',
						helperScript.code,
						helperScript.lang,
						{ ...args, filterText, _ENTRYPOINT_OVERRIDE: entrypoint },
						undefined,
						undefined,
						undefined,
						cb
					)
				: resultJobLoader?.runScriptByHash(
						helperScript?.hash ?? 'NO_HASH',
						{ ...args, filterText, _ENTRYPOINT_OVERRIDE: entrypoint },
						cb
					)
		})
	}

	let _items = usePromise(getItemsFromOptions)
	let items = $derived(_items.value)
	$effect(() => {
		;[args, name, entrypoint, helperScript, filterText]
		untrack(() => _items.refresh())
	})

	let resultJobLoader: JobLoader | undefined = $state()
	let filterText: string = $state('')
	let open: boolean = $state(false)
</script>

{#if helperScript}
	<JobLoader bind:this={resultJobLoader} />

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
				error: <Tooltip>{JSON.stringify(_items.error)}</Tooltip>
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
