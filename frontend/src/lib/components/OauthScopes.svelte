<script lang="ts">
	import { Button } from './common'
	import { Minus, Plus } from 'lucide-svelte'

	export let scopes: string[] = []
</script>

{#if scopes && Array.isArray(scopes)}
	{#each scopes as v}
		<div class="flex flex-row max-w-md mb-2">
			<input type="text" bind:value={v} />
			<Button
				variant="default"
				size="xs"
				btnClasses="mx-6"
				on:click={() => {
					scopes = scopes.filter((el) => el != v)
				}}
				startIcon={{ icon: Minus }}
				iconOnly
			/>
		</div>
	{/each}
{/if}

<div class="flex items-center mt-1">
	<Button
		variant="default"
		hover="yo"
		size="xs"
		startIcon={{ icon: Plus }}
		on:click={() => {
			scopes = (scopes ?? []).concat('')
		}}
	>
		Add item
	</Button>
	<span class="ml-2 text-xs text-primary font-normal">
		({(scopes ?? []).length} item{(scopes ?? []).length > 1 ? 's' : ''})
	</span>
</div>
