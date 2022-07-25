<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { formatValue, getTypeAsString } from '../flows/utils'
	import WarningMessage from './WarningMessage.svelte'

	export let json: Object
	export let level = 0
	export let isLast = true
	export let currentPath: string = ''

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

	$: collapsed = false

	function collapse() {
		collapsed = !collapsed
	}

	const dispatch = createEventDispatcher()

	function computeKey(key: string) {
		if (isArray) {
			if (currentPath === 'step') {
				return `${currentPath}(${key})?`
			}
			return `${currentPath}[${key}]`
		} else {
			if (currentPath) {
				return `${currentPath}.${key}`
			} else {
				return key
			}
		}
	}

	function selectProp(key: string) {
		dispatch('select', computeKey(key))
	}
</script>

{#if keys.length > 0}
	<span class:hidden={collapsed}>
		<span class="cursor-pointer hover:bg-slate-200" on:click={collapse}>{openBracket}</span>
		<ul>
			{#each keys as key, index}
				<li class={getTypeAsString(json[key]) !== 'object' ? 'hover:bg-sky-100 pt-1' : 'pt-1'}>
					{#if !isArray}
						<span class="key">{key}:</span>
					{/if}

					{#if getTypeAsString(json[key]) === 'object'}
						<svelte:self
							json={json[key]}
							level={level + 1}
							isLast={index === keys.length - 1}
							currentPath={computeKey(key)}
							on:select
						/>
					{:else}
						<span class="val {getTypeAsString(json[key])}">
							{#if formatValue(json[key]) === undefined}
								<WarningMessage />
							{:else}
								{formatValue(json[key])}
							{/if}

							{#if formatValue(json[key]) !== undefined}
								<button class="default-button-secondary" on:click={() => selectProp(key)}>
									Select
								</button>
							{/if}
						</span>
					{/if}
				</li>
			{/each}
		</ul>
		<span class="cursor-pointer hover:bg-slate-200" on:click={collapse}>{closeBracket}</span>
	</span>
	<span class="cursor-pointer hover:bg-slate-200" class:hidden={!collapsed} on:click={collapse}>
		{openBracket}{collapsedSymbol}{closeBracket}
	</span>
	{#if !isLast && collapsed}
		<span class="text-black">,</span>
	{/if}
{/if}

<style>
	ul {
		list-style: none;
		padding-left: 1rem;
		border-left: 1px dotted lightgray;
		@apply text-black;
	}

	.hidden {
		display: none;
	}
	.val {
		@apply text-black;
	}
	.val.string {
		@apply text-lime-600;
	}
	.val.number {
		@apply text-orange-600;
	}
	.val.boolean {
		@apply text-cyan-600;
	}
</style>
