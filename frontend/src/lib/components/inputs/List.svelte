<script lang="ts">
	import { faPlus } from '@fortawesome/free-solid-svg-icons'

	import Icon from 'svelte-awesome'
</script>

<div>
	<div>
		{#each value ?? [] as v}
			<div class="flex flex-row max-w-md mt-1">
				{#if itemsType?.type == 'number'}
					<input type="number" bind:value={v} />
				{:else if itemsType?.type == 'string' && itemsType?.contentEncoding == 'base64'}
					<input
						type="file"
						class="my-6"
						on:change={(x) => fileChanged(x, (val) => (v = val))}
						multiple={false}
					/>
				{:else}
					<input type="text" bind:value={v} />
				{/if}
				<button
					class="default-button-secondary mx-6"
					on:click={() => {
						value = value.filter((el) => el != v)
						if (value.length == 0) {
							value = undefined
						}
					}}
				>
					<Icon data={faMinus} class="mb-1" />
				</button>
			</div>
		{/each}
	</div>
	<button
		class="default-button-secondary mt-1"
		on:click={() => {
			if (value == undefined) {
				value = []
			}
			value = value.concat('')
		}}
	>
		<Icon data={faPlus} class="mr-2" />
		Add item
	</button>
	<span class="ml-2">
		{(value ?? []).length} item{(value ?? []).length > 1 ? 's' : ''}
	</span>
</div>
