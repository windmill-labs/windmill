<script lang="ts">
	import { Button } from './common'
	import { Minus, Plus } from 'lucide-svelte'

	export let extra_params: Record<string, string> = {}

	let extra_params_vec: [string, string][] = Object.entries(extra_params)

	function sync() {
		extra_params = Object.fromEntries(extra_params_vec)
	}
</script>

{#each extra_params_vec as o}
	<div class="flex flex-row max-w-md mb-2">
		<input type="text" on:keyup={sync} bind:value={o[0]} />
		<input type="text" on:keyup={sync} bind:value={o[1]} />
		<Button
			variant="border"
			color="red"
			size="xs"
			btnClasses="mx-6"
			on:click={() => {
				extra_params_vec = extra_params_vec.filter((e) => e[0] != o[0])
				sync()
			}}
			startIcon={{ icon: Minus }}
			iconOnly
		/>
	</div>
{/each}
<div class="flex items-center mt-1">
	<Button
		variant="border"
		color="blue"
		hover="yo"
		size="sm"
		endIcon={{ icon: Plus }}
		on:click={() => {
			extra_params_vec = extra_params_vec.concat([['key', 'value']])
			sync()
		}}
	>
		Add item
	</Button>
	<span class="ml-2 text-sm text-tertiary">
		({(extra_params_vec ?? []).length} item{(extra_params_vec ?? []).length > 1 ? 's' : ''})
	</span>
</div>
