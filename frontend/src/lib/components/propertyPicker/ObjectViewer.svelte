<script lang="ts">
	import { copyToClipboard, truncate } from '$lib/utils'

	import { createEventDispatcher } from 'svelte'
	import { computeKey } from './utils'
	import { NEVER_TESTED_THIS_FAR } from '../flows/models'
	import Portal from '$lib/components/Portal.svelte'
	import { Button } from '$lib/components/common'
	import { Download, PanelRightOpen } from 'lucide-svelte'
	import S3FilePicker from '../S3FilePicker.svelte'
	import { workspaceStore } from '$lib/stores'

	export let json: any
	export let level = 0
	export let currentPath: string = ''
	export let pureViewer = false
	export let collapsed = (level != 0 && level % 3 == 0) || Array.isArray(json)
	export let rawKey = false
	export let topBrackets = false
	export let allowCopy = true
	export let collapseLevel: number | undefined = undefined
	export let prefix = ''

	let s3FileViewer: S3FilePicker

	const collapsedSymbol = '...'
	$: keys = ['object', 's3object'].includes(getTypeAsString(json)) ? Object.keys(json) : []
	$: isArray = Array.isArray(json)
	$: openBracket = isArray ? '[' : '{'
	$: closeBracket = isArray ? ']' : '}'

	export function getTypeAsString(arg: any): string {
		if (arg === null) {
			return 'null'
		}
		if (arg === undefined) {
			return 'undefined'
		}
		if (Object.keys(arg).length === 1 && Object.keys(arg).includes('s3')) {
			return 's3object'
		}
		return typeof arg
	}

	function collapse() {
		collapsed = !collapsed
	}

	const dispatch = createEventDispatcher()

	function computeFullKey(key: string, rawKey: boolean) {
		if (rawKey) {
			return `${prefix}('${key}')`
		}
		const keyToSelect = computeKey(key, isArray, currentPath)
		const separator = !prefix || keyToSelect.startsWith('[') ? '' : '.'
		return prefix + separator + keyToSelect
	}

	function selectProp(key: string, value: any | undefined, clickedValue: boolean) {
		const fullKey = computeFullKey(key, rawKey)
		if (allowCopy) {
			if (pureViewer && clickedValue) {
				copyToClipboard(typeof value == 'string' ? value : JSON.stringify(value))
			} else {
				copyToClipboard(fullKey)
			}
		}
		dispatch('select', fullKey)
	}

	$: keyLimit = isArray ? 1 : 100

	$: fullyCollapsed = keys.length > 1 && collapsed
</script>

<Portal name="object-viewer">
	<S3FilePicker bind:this={s3FileViewer} readOnlyMode={true} />
</Portal>
{#if keys.length > 0}
	{#if !fullyCollapsed}
		<span>
			{#if level != 0 && keys.length > 1}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<!-- svelte-ignore a11y-no-static-element-interactions -->

				<Button
					color="light"
					size="xs2"
					variant="border"
					on:click={collapse}
					wrapperClasses="inline-flex w-fit"
					btnClasses="font-mono h-4 text-2xs px-1 font-thin text-primary rounded-[0.275rem]"
					>-</Button
				>
			{/if}
			{#if level == 0 && topBrackets}<span class="h-0">{openBracket}</span>{/if}
			<ul class={`w-full ${level === 0 ? 'border-none' : 'pl-2 border-l border-dotted'}`}>
				{#each keys.length > keyLimit ? keys.slice(0, keyLimit) : keys as key, index (key)}
					<li>
						<div class="inline-flex items-baseline">
							<Button
								on:click={() => selectProp(key, undefined, false)}
								size="xs2"
								color="light"
								variant="border"
								wrapperClasses="inline-flex p-0 whitespace-nowrap w-fit"
								btnClasses="font-mono h-4 text-2xs font-thin px-1 rounded-[0.275rem]"
								title={computeFullKey(key, rawKey)}
							>
								<span class={pureViewer ? 'cursor-auto' : ''}>{!isArray ? key : index} </span>
							</Button>
							<span class="text-2xs text-tertiary">:</span>
						</div>
						{#if getTypeAsString(json[key]) === 'object'}
							<svelte:self
								json={json[key]}
								level={level + 1}
								currentPath={computeFullKey(key, rawKey)}
								{pureViewer}
								{allowCopy}
								on:select
								{collapseLevel}
								collapsed={collapseLevel !== undefined ? level + 1 >= collapseLevel : undefined}
							/>
						{:else}
							<button
								class="val text-left {pureViewer
									? 'cursor-auto'
									: ''} rounded px-1 {getTypeAsString(json[key])}"
								on:click={() => {
									selectProp(key, json[key], true)
								}}
								title={JSON.stringify(json[key])}
								disabled={false}
							>
								{#if json[key] === NEVER_TESTED_THIS_FAR}
									<span class="text-2xs text-tertiary font-normal">
										Test the flow to see a value
									</span>
								{:else if json[key] == undefined}
									<span class="text-2xs">undefined</span>
								{:else if json[key] == null}
									<span class="text-2xs">null</span>
								{:else if typeof json[key] == 'string'}
									<span class="text-2xs">"{truncate(json[key], 200)}"</span>
								{:else}
									<span class="text-2xs">
										{truncate(JSON.stringify(json[key]), 200)}
									</span>
								{/if}
							</button>
						{/if}
					</li>
				{/each}
				{#if keys.length > keyLimit}
					{@const increment = Math.min(100, keys.length - keyLimit)}
					<button on:click={() => (keyLimit += increment)} class="text-2xs px-2 text-secondary">
						{keyLimit}/{keys.length}: Load {increment} more...
					</button>
				{/if}
			</ul>
			{#if level == 0 && topBrackets}
				<div class="flex">
					<span class="h-0">{closeBracket}</span>
					{#if getTypeAsString(json) === 's3object'}
						<a
							class="text-secondary underline font-semibold text-2xs whitespace-nowrap ml-1 w-fit"
							href={`/api/w/${$workspaceStore}/job_helpers/download_s3_file?file_key=${json?.s3}${
								json?.storage ? `&storage=${json.storage}` : ''
							}`}
							download={json?.s3.split('/').pop() ?? 'unnamed_download.file'}
						>
							<span class="flex items-center gap-1"><Download size={12} />download</span>
						</a>
						<button
							class="text-secondary underline text-2xs whitespace-nowrap ml-1"
							on:click={() => {
								s3FileViewer?.open?.(json)
							}}
							><span class="flex items-center gap-1"><PanelRightOpen size={12} />open preview</span>
						</button>
					{/if}
				</div>
			{/if}
		</span>
	{/if}

	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<!-- svelte-ignore a11y-no-static-element-interactions -->

	{#if fullyCollapsed}
		<div class="inline-flex relative">
			<Button
				color="light"
				size="xs2"
				variant="border"
				on:click={collapse}
				wrapperClasses="inline-flex w-fit"
				btnClasses="font-normal rounded-[0.275rem] h-4 px-1"
			>
				{openBracket}{collapsedSymbol}{closeBracket}
			</Button>
		</div>
	{/if}
{:else if topBrackets}
	<span class="text-primary">{openBracket}{closeBracket}</span>
{:else if json == undefined}
	<span class="text-tertiary text-2xs ml-2">undefined</span>
{:else}
	<span class="text-tertiary text-2xs ml-2">No items</span>
{/if}

<style lang="postcss">
	ul {
		list-style: none;
		@apply text-sm;
	}

	.val.undefined {
		@apply text-tertiary;
	}
	.val.null {
		@apply text-tertiary;
	}
	.val.string {
		@apply text-green-600 dark:text-green-400/80;
	}

	.val.number {
		@apply text-orange-600 dark:text-orange-400/90;
		@apply font-mono;
	}
	.val.boolean {
		@apply text-blue-600 dark:text-blue-400/90;
	}
</style>
