<script lang="ts">
	import { Paintbrush2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { addWhitespaceBeforeCapitals } from '../../../../utils'
	import { Button, ClearableInput } from '../../../common'
	import type { ComponentCssProperty } from '../../types'
	import QuickStyleMenu from './QuickStyleMenu.svelte'
	import type { StylePropertyKey } from './quickStyleProperties'

	export let name: string
	export let value: ComponentCssProperty = {}
	export let forceStyle: boolean = false
	export let forceClass: boolean = false
	export let quickStyleProperties: StylePropertyKey[] | undefined = undefined
	const dispatch = createEventDispatcher()
	let isQuickMenuOpen = false

	$: dispatch('change', value)
</script>

<div class="sticky top-0 z-20 bg-white text-sm font-semibold text-gray-500 capitalize pt-2 pb-1">
	{addWhitespaceBeforeCapitals(name)}
</div>
{#if value}
	<div class="border-l border-gray-400/80 py-1 pl-3.5 ml-0.5">
		{#if value.style !== undefined || forceStyle}
			<div class="pb-2">
				<!-- svelte-ignore a11y-label-has-associated-control -->
				<label class="block">
					<div class="text-xs font-medium pb-0.5"> Style </div>
					<div class="flex gap-1">
						<div class="relative grow">
							<ClearableInput bind:value={value.style} />
						</div>
						{#if quickStyleProperties?.length}
							<Button
								variant="border"
								color="light"
								size="xs"
								btnClasses="!p-1 !w-[34px] !h-[34px]"
								aria-label="Toggle quick style menu"
								title="Toggle quick style menu"
								on:click={() => (isQuickMenuOpen = !isQuickMenuOpen)}
							>
								<Paintbrush2 size={18} />
							</Button>
						{/if}
					</div>
				</label>
				{#if quickStyleProperties?.length && isQuickMenuOpen}
					<div transition:fade|local={{ duration: 200 }} class="w-full pt-1">
						<QuickStyleMenu bind:value={value.style} properties={quickStyleProperties} />
					</div>
				{/if}
			</div>
		{/if}
		{#if value.class !== undefined || forceClass}
			<!-- svelte-ignore a11y-label-has-associated-control -->
			<label class="block">
				<div class="text-xs font-medium pb-0.5"> Tailwind classes </div>
				<div class="relative">
					<ClearableInput bind:value={value.class} />
				</div>
			</label>
		{/if}
	</div>
{/if}
