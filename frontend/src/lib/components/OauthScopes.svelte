<script lang="ts">
	import { faMinus, faPlus } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { Button } from './common'

	export let scopes: string[] = []
</script>

{#each scopes as v}
	<div class="flex flex-row max-w-md mb-2">
		<input type="text" bind:value={v} />
		<Button
			variant="border"
			color="red"
			size="xs"
			btnClasses="mx-6"
			on:click={() => {
				scopes = scopes.filter((el) => el != v)
			}}
		>
			<Icon data={faMinus} />
		</Button>
	</div>
{/each}
<div class="flex items-center mt-1">
	<Button
		variant="border"
		color="blue"
		hover="yo"
		size="sm"
		endIcon={{ icon: faPlus }}
		on:click={() => {
			scopes = scopes.concat('')
		}}
	>
		Add item
	</Button>
	<span class="ml-2 text-sm text-tertiary">
		({(scopes ?? []).length} item{(scopes ?? []).length > 1 ? 's' : ''})
	</span>
</div>
