<script lang="ts">
	import ObjectViewer from './ObjectViewer.svelte'

	import { copyToClipboard, truncate } from '$lib/utils'

	import { createEventDispatcher } from 'svelte'
	import { computeKey, keepByKeyOrValue } from './utils'
	import { NEVER_TESTED_THIS_FAR } from '../flows/models'
	import Portal from '$lib/components/Portal.svelte'
	import { Button } from '$lib/components/common'
	import { Download, PanelRightOpen, Search, TriangleAlertIcon, X } from 'lucide-svelte'
	import S3FilePicker from '../S3FilePicker.svelte'
	import { workspaceStore } from '$lib/stores'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import Popover from '../Popover.svelte'

	interface Props {
		json: any
		level?: number
		currentPath?: string
		pureViewer?: boolean
		collapsed?: any
		rawKey?: boolean
		topBrackets?: boolean
		allowCopy?: boolean
		collapseLevel?: number | undefined
		prefix?: string
		expandedEvenOnLevel0?: string | undefined
		connecting?: boolean
		small?: boolean
	}

	let {
		json,
		level = 0,
		currentPath = '',
		pureViewer = false,
		collapsed = $bindable((level != 0 && level % 3 == 0) || (Array.isArray(json) && level != 0)),
		rawKey = false,
		topBrackets = false,
		allowCopy = true,
		collapseLevel = undefined,
		prefix = '',
		expandedEvenOnLevel0 = undefined,
		connecting = false,
		small = false
	}: Props = $props()

	let jsonFiltered = $state(json)

	let search = $state('')
	let searchOpen = $state(false)

	function onJsonChange(json: any) {
		return searchOpen && search ? keepByKeyOrValue(json, search) : json
	}

	let searchTimeout: NodeJS.Timeout | undefined = undefined
	function onSearch() {
		if (searchTimeout) {
			clearTimeout(searchTimeout)
		}
		searchTimeout = setTimeout(() => {
			jsonFiltered = search ? keepByKeyOrValue(json, search) : json
		}, 100)
	}

	let s3FileViewer: S3FilePicker | undefined = $state()
	let hoveredKey: string | null = $state(null)

	const collapsedSymbol = '...'

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

	function clearSearch() {
		searchOpen = false
		jsonFiltered = json
		search = ''
	}
	$effect(() => {
		jsonFiltered = onJsonChange(json)
	})
	$effect(() => {
		search != undefined && searchOpen && onSearch()
	})
	let keys = $derived(
		['object', 's3object'].includes(getTypeAsString(jsonFiltered)) ? Object.keys(jsonFiltered) : []
	)
	let isArray = $derived(Array.isArray(jsonFiltered))
	let openBracket = $derived(isArray ? '[' : '{')
	let closeBracket = $derived(isArray ? ']' : '}')
	let keyLimit = $derived(isArray ? 5 : 100)
	let fullyCollapsed = $derived(keys.length > 1 && collapsed)
</script>

{#snippet renderScalar(k: string, v: any)}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<span
		class="val inline text-left {pureViewer ? 'cursor-auto' : ''} rounded pl-0.5 {getTypeAsString(
			v
		)}"
		onclick={() => {
			selectProp(k, v, true)
		}}
		title={JSON.stringify(v)}
	>
		{#if v === NEVER_TESTED_THIS_FAR}
			<span class="{small ? 'text-[10px]' : 'text-2xs'} text-tertiary font-normal">
				Test the flow to see a value
			</span>
		{:else if v == undefined}
			<span class={small ? 'text-[10px]' : 'text-2xs'}>undefined</span>
		{:else if v == null}
			<span class={small ? 'text-[10px]' : 'text-2xs'}>null</span>
		{:else if typeof v == 'string'}
			<span class={small ? 'text-[10px]' : 'text-2xs'}>"{truncate(v, 200)}"</span>
		{:else if typeof v == 'number' && Number.isInteger(v) && !Number.isSafeInteger(v)}
			<span class="inline-flex flex-row gap-1 items-center {small ? 'text-[10px]' : 'text-2xs'}">
				{truncate(JSON.stringify(v), 200)}
				<Popover>
					<TriangleAlertIcon size={14} class="text-yellow-500 mb-0.5" />
					{#snippet text()}
						This number is too large for the frontend to handle correctly and may be rounded.
					{/snippet}
				</Popover>
			</span>
		{:else}
			<span class={small ? 'text-[10px]' : 'text-2xs'}>
				{truncate(JSON.stringify(v), 200)}
			</span>
		{/if}
	</span>
{/snippet}

<Portal name="object-viewer">
	<S3FilePicker bind:this={s3FileViewer} readOnlyMode={true} />
</Portal>
{#if level == 0}
	<div class="float-right">
		{#if searchOpen}
			<div class="px-1 relative">
				<input
					onkeydown={(event) => {
						if ((event as KeyboardEvent)?.key === 'Escape') {
							clearSearch()
						}
						event.stopPropagation()
					}}
					type="text"
					class="!h-6 {small ? '!text-[10px]' : '!text-2xs'} mt-0.5"
					bind:value={search}
					placeholder="Search..."
				/>
				<button
					class="absolute right-2 top-1 rounded-full hover:bg-surface-hover focus:bg-surface-hover text-secondary p-0.5"
					onclick={() => {
						clearSearch()
					}}><X size={12} /></button
				>
			</div>
		{:else}
			<Button
				color="light"
				size="xs3"
				iconOnly
				btnClasses="text-tertiary hover:text-primary"
				startIcon={{ icon: Search }}
				on:click={() => (searchOpen = true)}
			></Button>
		{/if}
	</div>
{/if}

{#if keys.length > 0}
	{#if !fullyCollapsed}
		<span>
			{#if level != 0 && keys.length > 1}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->

				<Button
					color="light"
					size="xs2"
					variant="border"
					on:click={collapse}
					wrapperClasses="!inline-flex w-fit"
					btnClasses="font-mono h-4 {small
						? 'text-[10px]'
						: 'text-2xs'} px-1 font-thin text-primary rounded-[0.275rem]">-</Button
				>
			{/if}
			{#if level == 0 && topBrackets}<span class="text-tertiary">{openBracket}</span>{/if}
			<ul
				class={`w-full ${
					level === 0 ? `border-none ${topBrackets ? 'pl-2' : ''}` : 'pl-2 border-l border-dotted'
				}`}
			>
				{#each keys.length > keyLimit ? keys.slice(0, keyLimit) : keys as key, index (key)}
					<li>
						<AnimatedButton
							animate={connecting && hoveredKey === key}
							marginWidth="1px"
							wrapperClasses="inline-flex h-fit w-fit items-center"
							baseRadius="0.275rem"
							animationDuration="2s"
						>
							<Button
								on:click={() => selectProp(key, undefined, false)}
								on:mouseenter={() => {
									hoveredKey = key
								}}
								on:mouseleave={() => (hoveredKey = null)}
								size="xs2"
								color="light"
								variant="border"
								wrapperClasses="p-0 whitespace-nowrap w-fit"
								btnClasses="font-mono h-4 py-1 {small
									? 'text-[10px]'
									: 'text-2xs'} font-thin px-1 rounded-[0.275rem]"
								title={computeFullKey(key, rawKey)}
							>
								<span class={pureViewer ? 'cursor-auto' : ''}>{!isArray ? key : index} </span>
							</Button>
						</AnimatedButton>
						<span class="{small ? 'text-[10px]' : 'text-2xs'} -ml-0.5 text-tertiary">:</span>

						{#if getTypeAsString(jsonFiltered[key]) === 'object'}
							<ObjectViewer
								{connecting}
								json={jsonFiltered[key]}
								level={level + 1}
								currentPath={computeFullKey(key, rawKey)}
								{pureViewer}
								{allowCopy}
								on:select
								{collapseLevel}
								collapsed={collapseLevel !== undefined
									? level + 1 >= collapseLevel && key != expandedEvenOnLevel0
									: undefined}
								{small}
							/>
						{:else}
							{@render renderScalar(key, jsonFiltered[key])}
						{/if}
					</li>
				{/each}
				{#if keys.length > keyLimit}
					{@const increment = Math.min(100, keys.length - keyLimit)}
					<button
						onclick={() => (keyLimit += increment)}
						class="{small ? 'text-[10px]' : 'text-2xs'} px-2 text-secondary"
					>
						{keyLimit}/{keys.length}: Load {increment} more...
					</button>
				{/if}
			</ul>
			{#if level == 0 && topBrackets}
				<div class="flex">
					<span class="text-tertiary">{closeBracket}</span>
					{#if getTypeAsString(jsonFiltered) === 's3object'}
						<a
							class="text-secondary underline font-semibold {small
								? 'text-[10px]'
								: 'text-2xs'} whitespace-nowrap ml-1 w-fit"
							href={`/api/w/${$workspaceStore}/job_helpers/download_s3_file?file_key=${encodeURIComponent(
								jsonFiltered?.s3 ?? ''
							)}${jsonFiltered?.storage ? `&storage=${jsonFiltered.storage}` : ''}`}
							download={jsonFiltered?.s3.split('/').pop() ?? 'unnamed_download.file'}
						>
							<span class="flex items-center gap-1"><Download size={12} />download</span>
						</a>
						<button
							class="text-secondary underline {small
								? 'text-[10px]'
								: 'text-2xs'} whitespace-nowrap ml-1"
							onclick={() => {
								s3FileViewer?.open?.(jsonFiltered)
							}}
							><span class="flex items-center gap-1"><PanelRightOpen size={12} />open preview</span>
						</button>
					{/if}
				</div>
			{/if}
		</span>
	{/if}

	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->

	{#if fullyCollapsed}
		<span>
			<Button
				color="light"
				size="xs2"
				variant="border"
				on:click={collapse}
				wrapperClasses="!inline-flex w-fit"
				btnClasses="h-4 text-[9px] px-1  text-primary rounded-[0.275rem]"
			>
				{openBracket}{collapsedSymbol}{closeBracket}
			</Button>
		</span>
	{/if}
{:else if topBrackets}
	<span class="text-primary">{openBracket}{closeBracket}</span>
{:else if jsonFiltered == undefined}
	<span class="text-tertiary {small ? 'text-[10px]' : 'text-2xs'} ml-2">undefined</span>
{:else if typeof jsonFiltered != 'object'}
	{@render renderScalar('', jsonFiltered)}
{:else}
	<span class="text-tertiary {small ? 'text-[10px]' : 'text-2xs'} ml-2">No items</span>
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
