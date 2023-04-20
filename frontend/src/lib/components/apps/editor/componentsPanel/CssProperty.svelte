<script lang="ts">
	import { Paintbrush2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { addWhitespaceBeforeCapitals } from '../../../../utils'
	import { Button, ClearableInput } from '../../../common'
	import Popover from '../../../Popover.svelte'
	import type { ComponentCssProperty } from '../../types'
	import type { TypedComponent } from '../component'
	import QuickStyleMenu from './QuickStyleMenu.svelte'
	import type { PropertyGroup } from './quickStyleProperties'

	export let name: string
	export let value: ComponentCssProperty = {}
	export let forceStyle: boolean = false
	export let forceClass: boolean = false
	export let quickStyleProperties: PropertyGroup[] | undefined = undefined
	export let componentType: TypedComponent['type'] | undefined = undefined
	const dispatch = createEventDispatcher()
	let isQuickMenuOpen = false

	$: dispatch('change', value)

	function toggleQuickMenu() {
		isQuickMenuOpen = !isQuickMenuOpen
	}
</script>

<div
	class="sticky top-0 z-20 text-lg bg-gray-100 font-semibold lowercase leading-none [font-variant:small-caps] text-gray-700 px-3 pb-1 mt-4 mb-1"
>
	{addWhitespaceBeforeCapitals(name)}
</div>
{#if value}
	<div class="px-3">
		{#if value.style !== undefined || forceStyle}
			<div class="pb-2">
				<!-- svelte-ignore a11y-label-has-associated-control -->
				<label class="block">
					<div class="text-sm font-medium text-gray-600 pb-0.5"> Style </div>
					<div class="flex gap-1">
						<div class="relative grow">
							<ClearableInput
								bind:value={value.style}
								type="textarea"
								wrapperClass="h-full min-h-[72px]"
								inputClass="h-full"
							/>
						</div>
						<div class="flex flex-col gap-1">
							{#if quickStyleProperties?.length}
								<Popover placement="bottom" notClickable disapperTimeout={0}>
									<Button
										variant="border"
										color="light"
										size="xs"
										btnClasses="!p-1 !w-[34px] !h-[34px] {isQuickMenuOpen
											? '!bg-gray-200/60 hover:!bg-gray-200 focus:!bg-gray-200'
											: ''}"
										aria-label="{isQuickMenuOpen ? 'Close' : 'Open'} styling menu"
										on:click={toggleQuickMenu}
									>
										<Paintbrush2 size={18} />
									</Button>
									<svelte:fragment slot="text">
										{isQuickMenuOpen ? 'Close' : 'Open'} styling menu
									</svelte:fragment>
								</Popover>
							{/if}
						</div>
					</div>
				</label>
				{#if quickStyleProperties?.length && isQuickMenuOpen}
					<div transition:fade|local={{ duration: 200 }} class="w-full pt-1">
						<QuickStyleMenu
							bind:value={value.style}
							properties={quickStyleProperties}
							{componentType}
							componentProperty={name}
						/>
					</div>
				{/if}
			</div>
		{/if}
		{#if value.class !== undefined || forceClass}
			<!-- svelte-ignore a11y-label-has-associated-control -->
			<label class="block">
				<div class="text-sm font-medium text-gray-600 pb-0.5"> Tailwind classes </div>
				<div class="relative">
					<ClearableInput bind:value={value.class} />
				</div>
			</label>
		{/if}
	</div>
{/if}
