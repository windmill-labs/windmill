<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let json: Object
	export let depth = Infinity
	export let level = 0
	export let isLast = true

	const collapsedSymbol = '...'
	const dispatch = createEventDispatcher()

	function getType(arg: any): string {
		if (arg === null) {
			return 'null'
		}
		return typeof arg
	}

	let items: string | any[]
	let isArray: boolean
	let openBracket: string
	let closeBracket: string

	$: {
		items = getType(json) === 'object' ? Object.keys(json) : []
		isArray = Array.isArray(json)
		openBracket = isArray ? '[' : '{'
		closeBracket = isArray ? ']' : '}'
	}

	$: collapsed = depth < level

	function format(arg: any) {
		if (getType(arg) === 'string') {
			return `"${arg}"`
		}
		return arg
	}

	function collapse() {
		collapsed = !collapsed
	}
</script>

{#if items.length > 0}
	<span class:hidden={collapsed}>
		<span class="bracket" on:click={collapse}>{openBracket}</span>
		<ul>
			{#each items as item, index}
				<li class={getType(json[item]) !== 'object' ? 'hover:bg-sky-100 py-2' : 'py-2'}>
					{#if !isArray}
						<span class="key">{item}:</span>
					{/if}
					{#if getType(json[item]) === 'object'}
						<svelte:self
							json={json[item]}
							{depth}
							level={level + 1}
							isLast={index === items.length - 1}
						/>
					{:else}
						<span class="val {getType(json[item])}">
							{format(json[item])}
							<button class="default-button" on:click={() => dispatch('change', item)}>
								Select property: {item}
							</button>
							{#if index < items.length - 1}
								<span class="comma">,</span>
							{/if}
						</span>
					{/if}
				</li>
			{/each}
		</ul>
		<span class="bracket" on:click={collapse}>{closeBracket}</span>
		{#if !isLast}
			<span class="comma">,</span>
		{/if}
	</span>
	<span class="bracket" class:hidden={!collapsed} on:click={collapse}>
		{openBracket}{collapsedSymbol}{closeBracket}
	</span>
	{#if !isLast && collapsed}
		<span class="comma">,</span>
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
	.bracket {
		cursor: pointer;
	}
	.bracket:hover {
		background-color: lightgray;
	}
	.comma {
		@apply text-black;
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
