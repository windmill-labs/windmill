<script lang="ts">
	import { Button } from './common'

	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object'
				contentEncoding?: 'base64'
				enum?: string[]
		  }
		| undefined

	let selected: 'string' | 'number' | 'object' | 'bytes' | 'enum' | undefined =
		itemsType?.type != 'string'
			? itemsType?.type
			: Array.isArray(itemsType?.enum)
			? 'enum'
			: 'string'
</script>

<select
	bind:value={selected}
	on:change={() => {
		if (selected == 'enum') {
			itemsType = { type: 'string', enum: [] }
		} else if (selected == 'string') {
			itemsType = { type: 'string' }
		} else if (selected == 'number') {
			itemsType = { type: 'number' }
		} else if (selected == 'object') {
			itemsType = { type: 'object' }
		} else if (selected == 'bytes') {
			itemsType = { type: 'string', contentEncoding: 'base64' }
		} else {
			itemsType = undefined
		}
	}}
>
	<option value="string"> Items are strings</option>
	<option value="enum">Items are strings from an enum</option>
	<option value="object"> Items are objects (JSON)</option>
	<option value="number">Items are numbers</option>
	<option value="bytes">Items are bytes</option>
</select>
{#if Array.isArray(itemsType?.enum)}
	<div class="pt-1" />
	<label for="input" class="mb-2 text-secondary text-xs">
		Enums
		<div class="flex flex-col gap-1">
			{#each itemsType?.enum || [] as e}
				<div class="flex flex-row max-w-md">
					<input id="input" type="text" bind:value={e} />
					<Button
						size="sm"
						btnClasses="ml-6"
						on:click={() => {
							if (itemsType?.enum) {
								itemsType.enum = (itemsType.enum || []).filter((el) => el !== e)
							}
						}}>-</Button
					>
				</div>
			{/each}
		</div>
		<div class="flex flex-row my-1">
			<Button
				size="sm"
				on:click={() => {
					if (itemsType?.enum) {
						itemsType.enum = itemsType.enum ? itemsType.enum.concat('') : ['']
					}
				}}>+</Button
			>
			<Button
				variant="border"
				size="sm"
				btnClasses="ml-2"
				on:click={() => itemsType?.enum && (itemsType.enum = undefined)}
			>
				Clear
			</Button>
		</div>
	</label>
{/if}
