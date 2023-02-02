<script lang="ts">
	import { copyToClipboard, pluralize, truncate } from '$lib/utils'

	import { createEventDispatcher } from 'svelte'
	import { Badge } from '../common'
	import { NEVER_TESTED_THIS_FAR } from '../flows/utils'
	import { getTypeAsString } from '../flows/utils'
	import { computeKey } from './utils'
	import WarningMessage from './WarningMessage.svelte'

	export let json: Object
	export let level = 0
	export let isLast = true
	export let currentPath: string = ''
	export let pureViewer = false
	export let collapsed = level == 3 || Array.isArray(json)
	export let rawKey = false
	export let topBrackets = false
	export let topLevelNode = false
	export let allowCopy = true

	const collapsedSymbol = '...'
	let keys: string | any[]
	let isArray: boolean
	let openBracket: string
	let closeBracket: string

	$: {
		keys = getTypeAsString(json) === 'object' ? Object.keys(json) : []
		isArray = Array.isArray(json)
		openBracket = isArray ? '[' : '{'
		closeBracket = isArray ? ']' : '}'
	}

	function collapse() {
		collapsed = !collapsed
	}

	const dispatch = createEventDispatcher()

	function selectProp(key: string, value: any) {
		if (pureViewer && allowCopy) {
			copyToClipboard(value)
		}
		dispatch('select', rawKey ? key : computeKey(key, isArray, currentPath))
	}
</script>

{#if keys.length > 0}
	<span class:hidden={collapsed}>
		{#if level != 0}
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<span
				class="cursor-pointer border border-gray-300 hover:bg-gray-200 px-1 rounded"
				on:click={collapse}
			>
				-
			</span>
		{/if}
		{#if level == 0 && topBrackets}<span class="h-0">{openBracket}</span>{/if}
		<ul class="w-full">
			{#each keys as key, index}
				<li class="pt-1">
					<button on:click={() => selectProp(key, key)} class="whitespace-nowrap">
						{#if topLevelNode}
							<Badge baseClass="border border-blue-600" color="indigo">{key}</Badge>
						{:else}
							<span
								class="key {pureViewer
									? 'cursor-auto'
									: 'border border-gray-300'} font-normal rounded px-1 hover:bg-blue-100"
							>
								{!isArray ? key : index}</span
							>
						{/if}:
					</button>

					{#if getTypeAsString(json[key]) === 'object'}
						<svelte:self
							json={json[key]}
							level={level + 1}
							isLast={index === keys.length - 1}
							currentPath={computeKey(key, isArray, currentPath)}
							{pureViewer}
							on:select
						/>
					{:else}
						<button
							class="val {pureViewer
								? 'cursor-auto'
								: ''} rounded hover:bg-blue-100 {getTypeAsString(json[key])}"
							on:click={() => selectProp(key, json[key])}
						>
							{#if json[key] === NEVER_TESTED_THIS_FAR}
								<WarningMessage />
							{:else if json[key] == undefined}
								<span class="text-2xs">undefined</span>
							{:else if typeof json[key] == 'string'}
								<span title={json[key]} class="text-2xs">"{truncate(json[key], 200)}"</span>
							{:else}
								<span title={JSON.stringify(json[key])} class="text-2xs"
									>{truncate(JSON.stringify(json[key]), 200)}</span
								>
							{/if}
						</button>
					{/if}
				</li>
			{/each}
		</ul>
		{#if level == 0 && topBrackets}<span class="h-0">{closeBracket}</span>{/if}
	</span>
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<span
		class="border border-blue-600 rounded px-1 cursor-pointer hover:bg-gray-200"
		class:hidden={!collapsed}
		on:click={collapse}
	>
		{openBracket}{collapsedSymbol}{closeBracket}
	</span>
	{#if collapsed}
		<span class="text-gray-500 text-xs">
			{pluralize(Object.keys(json).length, Array.isArray(json) ? 'item' : 'key')}
		</span>
	{/if}
	{#if !isLast && collapsed}
		<span class="text-black">,</span>
	{/if}
{:else if topBrackets}
	<span class="text-black">{openBracket}{closeBracket}</span>
{:else}
	<span class="text-gray-600 text-xs">No items</span>
{/if}

<style>
	ul {
		list-style: none;
		padding-left: 1rem;
		border-left: 1px dotted lightgray;
		@apply text-sm;
	}

	.val {
		/* @apply font-black; */
	}
	.val.undefined {
		@apply text-red-500;
	}
	.val.null {
		@apply text-red-500;
	}
	.val.string {
		@apply text-green-600;
	}
	.val.number {
		@apply text-orange-600;
	}
	.val.boolean {
		@apply text-blue-600;
	}
</style>
