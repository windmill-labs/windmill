<script lang="ts">
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import type { Script } from '$lib/gen'
	import { deepEqual } from 'fast-equals'
	import SelectLegacy from './apps/svelte-select/lib/SelectLegacy.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import ResultJobLoader from './ResultJobLoader.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Loader2 } from 'lucide-svelte'

	export let value: any = undefined
	export let helperScript:
		| { type: 'inline'; path?: string; lang: Script['language']; code: string }
		| { type: 'hash'; hash: string }
		| undefined = undefined
	export let entrypoint: string
	export let args: Record<string, any> = {}
	export let name: string

	let darkMode: boolean = false

	let rawCode = JSON.stringify(value, null, 2)
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
	async function getItemsFromOptions(text: string) {
		return new Promise((resolve, reject) => {
			let cb = {
				done(res) {
					if (!res || !Array.isArray(res)) {
						reject('Result was not an array')
						return
					}
					if (res.length == 0) {
						resolve([])
					}

					if (res.every((x) => typeof x == 'string')) {
						res = res.map((x) => ({
							label: x,
							value: x
						}))
					} else if (res.find((x) => validSelectObject(x) != undefined)) {
						reject(validSelectObject(res.find((x) => validSelectObject(x) != undefined)))
					} else {
						if (text != undefined && text != '') {
							res = res.filter((x) => x['label'].includes(text))
						}
						resolve(res)
					}
				},
				cancel() {
					reject()
				},
				error(err) {
					reject(err)
				}
			}
			helperScript?.type == 'inline'
				? resultJobLoader?.runPreview(
						helperScript?.path ?? 'NO_PATH',
						helperScript.code,
						helperScript.lang,
						{ ...args, text, _ENTRYPOINT_OVERRIDE: entrypoint },
						undefined,
						cb
					)
				: resultJobLoader?.runScriptByHash(
						helperScript?.hash ?? 'NO_HASH',
						{
							...args,
							text,
							_ENTRYPOINT_OVERRIDE: entrypoint
						},
						cb
					)
		})
	}

	let lastArgs = structuredClone({ ...args, [name]: undefined })
	$: (entrypoint || helperScript) && refreshOptions()

	$: args && changeArgs()

	let timeout: NodeJS.Timeout | undefined = undefined
	function changeArgs() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(() => {
			let argsWithoutSelf = { ...args, [name]: undefined }
			if (deepEqual(argsWithoutSelf, lastArgs)) {
				return
			}
			refreshOptions()
			lastArgs = structuredClone(argsWithoutSelf)
			timeout = undefined
		}, 1000)
	}

	function refreshOptions() {
		error = undefined
		renderCount += 1
	}

	let error: string | undefined = undefined
	let resultJobLoader: ResultJobLoader
	let renderCount = 0
</script>

{#if helperScript}
	<DarkModeObserver bind:darkMode />
	<ResultJobLoader bind:this={resultJobLoader} />

	<div class="w-full flex-col flex">
		<div class="w-full">
			{#key renderCount}
				<SelectLegacy
					on:error={(e) => {
						error = e.detail.details
					}}
					on:change={(e) => {
						value = e.detail.value
					}}
					{value}
					computeOnClick={value == undefined}
					loadOptions={getItemsFromOptions}
					inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
					containerStyles={darkMode
						? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
						: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
				/>
			{/key}
		</div>
		{#if error}
			<div class="text-red-400 text-2xs">error: <Tooltip>{JSON.stringify(error)}</Tooltip></div>
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
			<Module.default code={rawCode} bind:value />
		{/await}
	</div>
{/if}
