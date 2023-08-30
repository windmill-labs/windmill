<script lang="ts">
	import { MoveLeft, MoveRight, Paintbrush2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { addWhitespaceBeforeCapitals } from '../../../../utils'
	import { Button, ClearableInput } from '../../../common'
	import Popover from '../../../Popover.svelte'
	import type { ComponentCssProperty } from '../../types'
	import type { TypedComponent } from '../component'
	import QuickStyleMenu from './QuickStyleMenu.svelte'
	import type { PropertyGroup } from './quickStyleProperties'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	export let name: string
	export let value: ComponentCssProperty = {}
	export let forceStyle: boolean = false
	export let forceClass: boolean = false
	export let quickStyleProperties: PropertyGroup[] | undefined = undefined
	export let componentType: TypedComponent['type'] | undefined = undefined
	export let tooltip: string | undefined = undefined
	export let shouldDisplayLeft: boolean = false
	export let shouldDisplayRight: boolean = false

	const dispatch = createEventDispatcher()
	let isQuickMenuOpen = false

	$: dispatch('change', value)

	function toggleQuickMenu() {
		isQuickMenuOpen = !isQuickMenuOpen
	}
</script>

<div
	class="capitalize border-b flex justify-between items-center h-8 p-2 text-xs leading-6 font-bold"
>
	{addWhitespaceBeforeCapitals(name)}
	{#if shouldDisplayLeft}
		<Button
			color="dark"
			size="xs2"
			on:click={() => {
				dispatch('left')
			}}
		>
			<div class="flex flex-row gap-2 text-2xs items-center">
				<MoveLeft size={14} />
				Copy to local styling
			</div>
		</Button>
	{/if}
	{#if shouldDisplayRight}
		<Button
			color="dark"
			size="xs2"
			on:click={() => {
				dispatch('right')
			}}
		>
			<div class="flex flex-row gap-2 text-2xs items-center">
				Copy to global styling
				<MoveRight size={14} />
			</div>
		</Button>
	{/if}
</div>

{#if value}
	<div class="p-2">
		{#if tooltip}
			<div class="text-tertiary text-2xs py-2">{tooltip}</div>
		{/if}
		{#if value.style !== undefined || forceStyle}
			<div class="pb-2">
				<!-- svelte-ignore a11y-label-has-associated-control -->
				<label class="block mb-0.5 w-full">
					<div class="flex flex-row justify-between items-center w-full p-0.5">
						<div class="text-xs font-medium text-tertiary"> Plain CSS </div>
						<Badge color="blue">Overriden by local</Badge>
					</div>

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
								<Popover placement="bottom" notClickable disappearTimeout={0}>
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
			<label class="block mb-0.5">
				<div class="text-xs font-medium text-tertiary">
					Tailwind classes
					<Tooltip light documentationLink="https://tailwindcss.com/">
						Use any tailwind classes to style your component
					</Tooltip>
				</div>
				<div class="relative">
					<ClearableInput bind:value={value.class} />
				</div>
			</label>
		{/if}
	</div>
{/if}
