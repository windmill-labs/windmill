<script lang="ts">
	import { copyToClipboard, pluralize, truncate } from '$lib/utils'

	import { createEventDispatcher } from 'svelte'
	import { Badge } from '../common'
	import { computeKey } from './utils'
	import WarningMessage from './WarningMessage.svelte'
	import { NEVER_TESTED_THIS_FAR } from '../flows/models'
	import Portal from 'svelte-portal'
	import S3FilePicker from '../S3FilePicker.svelte'

	export let json: any
	export let level = 0
	export let currentPath: string = ''
	export let pureViewer = false
	export let collapsed = (level != 0 && level % 3 == 0) || Array.isArray(json)
	export let rawKey = false
	export let topBrackets = false
	export let topLevelNode = false
	export let allowCopy = true

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

	function selectProp(key: string, value: any | undefined = undefined) {
		if (pureViewer && allowCopy) {
			const valueToCopy = value !== undefined ? value : computeKey(key, isArray, currentPath)
			copyToClipboard(valueToCopy)
		}
		dispatch('select', rawKey ? key : computeKey(key, isArray, currentPath))
	}

	$: keyLimit = isArray ? 1 : 100
</script>

<Portal>
	<S3FilePicker
		bind:this={s3FileViewer}
		initialFileKey={json}
		selectedFileKey={json}
		readOnlyMode={true}
	/>
</Portal>

{#if keys.length > 0}
	{#if !collapsed}
		<span>
			{#if level != 0}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<!-- svelte-ignore a11y-no-static-element-interactions -->
				<span class="cursor-pointer border hover:bg-surface-hover px-1 rounded" on:click={collapse}>
					-
				</span>
			{/if}
			{#if level == 0 && topBrackets}<span class="h-0">{openBracket}</span>{/if}
			<ul class={`w-full pl-2 ${level === 0 ? 'border-none' : 'border-l border-dotted'}`}>
				{#each keys.length > keyLimit ? keys.slice(0, keyLimit) : keys as key, index (key)}
					<li>
						<button on:click={() => selectProp(key)} class="whitespace-nowrap">
							{#if topLevelNode}
								<Badge baseClass="border border-blue-600" color="indigo">{key}</Badge>
							{:else}
								<span
									class="key {pureViewer
										? 'cursor-auto'
										: 'border '} font-semibold rounded px-1 hover:bg-surface-hover text-2xs text-secondary"
								>
									{!isArray ? key : index}</span
								>
							{/if}:
						</button>

						{#if getTypeAsString(json[key]) === 'object'}
							<svelte:self
								json={json[key]}
								level={level + 1}
								currentPath={computeKey(key, isArray, currentPath)}
								{pureViewer}
								{allowCopy}
								on:select
							/>
						{:else}
							<button
								class="val text-left {pureViewer
									? 'cursor-auto'
									: ''} rounded px-1 hover:bg-blue-100 {getTypeAsString(json[key])}"
								on:click={() => selectProp(key, json[key])}
							>
								{#if json[key] === NEVER_TESTED_THIS_FAR}
									<WarningMessage />
								{:else if json[key] == undefined}
									<span class="text-2xs">undefined</span>
								{:else if json[key] == null}
									<span class="text-2xs">null</span>
								{:else if typeof json[key] == 'string'}
									<span title={json[key]} class="text-2xs">"{truncate(json[key], 200)}"</span>
								{:else}
									<span title={JSON.stringify(json[key])} class="text-2xs">
										{truncate(JSON.stringify(json[key]), 200)}
									</span>
								{/if}
							</button>
						{/if}
					</li>
				{/each}
				{#if keys.length > keyLimit}
					{@const increment = Math.min(100, keys.length - keyLimit)}
					<button on:click={() => (keyLimit += increment)} class="text-xs py-2 text-blue-600">
						{keyLimit}/{keys.length}: Load {increment} more...
					</button>
				{/if}
			</ul>
			{#if level == 0 && topBrackets}
				<span class="h-0">{closeBracket}</span>
				{#if getTypeAsString(json) === 's3object'}
					<button
						class="text-secondary underline text-2xs whitespace-nowrap ml-1"
						on:click={() => {
							s3FileViewer?.open?.()
						}}
						>s3 explorer
					</button>
				{/if}
			{/if}
		</span>
	{/if}

	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<span
		class="border border-blue-600 rounded px-1 cursor-pointer hover:bg-gray-200"
		class:hidden={!collapsed}
		on:click={collapse}
	>
		{openBracket}{collapsedSymbol}{closeBracket}
	</span>
	{#if collapsed}
		<span class="text-tertiary text-xs">
			{pluralize(Object.keys(json).length, Array.isArray(json) ? 'item' : 'key')}
		</span>
	{/if}
{:else if topBrackets}
	<span class="text-primary">{openBracket}{closeBracket}</span>
{:else if json == undefined}
	<span class="text-tertiary text-2xs ml-2">undefined</span>
{:else}
	<span class="text-tertiary text-2xs ml-2">No items ([])</span>
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
		@apply text-green-600;
	}

	.val.number {
		@apply text-orange-600;
		@apply font-mono;
	}
	.val.boolean {
		@apply text-blue-600;
	}
</style>
