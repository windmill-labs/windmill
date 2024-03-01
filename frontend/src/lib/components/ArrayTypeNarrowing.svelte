<script lang="ts">
	import { Plus, X } from 'lucide-svelte'
	import { Button } from './common'
	import { fade } from 'svelte/transition'

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
	id="array-type-narrowing"
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
				<div class="flex flex-row max-w-md gap-1 items-center">
					<input id="input" type="text" bind:value={e} />
					<div>
						<button
							transition:fade|local={{ duration: 100 }}
							class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
							on:click={() => {
								if (itemsType?.enum) {
									itemsType.enum = (itemsType.enum || []).filter((el) => el !== e)
								}
							}}
						>
							<X size={14} />
						</button>
					</div>
				</div>
			{/each}
		</div>
		<div class="flex flex-row mb-1 mt-2">
			<Button
				color="light"
				variant="border"
				size="sm"
				on:click={() => {
					if (itemsType?.enum) {
						let enum_ = itemsType.enum
						let choice = `choice ${enum_?.length ? enum_?.length + 1 : 1}`
						itemsType.enum = itemsType.enum ? itemsType.enum.concat(choice) : [choice]
					}
				}}
			>
				<Plus size={14} />
			</Button>
			<Button
				color="light"
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
