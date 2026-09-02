<script lang="ts">
	import { Button } from './common'
	import Checkbox from './common/checkbox/Checkbox.svelte'
	import { Minus, Plus } from 'lucide-svelte'

	interface Props {
		scopes?: string[]
		/** Scopes the provider is known to accept, offered as checkboxes. Anything
		 * not in this list stays editable as free text below them. */
		options?: string[]
	}

	let { scopes = $bindable(), options = [] }: Props = $props()

	$effect.pre(() => {
		if (!scopes) {
			scopes = []
		}
	})

	function toggle(option: string, on: boolean) {
		const current = scopes ?? []
		scopes = on
			? current.includes(option)
				? current
				: [...current, option]
			: current.filter((s) => s != option)
	}
</script>

{#if options.length > 0}
	<div class="flex flex-col gap-1 mb-2">
		{#each options as option (option)}
			<label class="flex items-center gap-2 text-xs">
				<Checkbox
					checked={scopes?.includes(option) ?? false}
					onChange={(e) => toggle(option, e.currentTarget.checked)}
				/>
				<span class="font-mono break-all">{option}</span>
			</label>
		{/each}
	</div>
	<span class="text-xs text-secondary">Custom scopes</span>
{/if}

{#if scopes && Array.isArray(scopes)}
	{#each scopes as v, i (i)}
		{#if !options.includes(v)}
			<div class="flex flex-row max-w-md mb-2">
				<input type="text" bind:value={scopes[i]} />
				<Button
					variant="default"
					size="xs"
					btnClasses="mx-6"
					on:click={() => {
						scopes = scopes?.filter((el) => el != v)
					}}
					startIcon={{ icon: Minus }}
					iconOnly
				/>
			</div>
		{/if}
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
