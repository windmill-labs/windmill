<script lang="ts">
	import { Forward, Paintbrush2 } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import { addWhitespaceBeforeCapitals, sendUserToast } from '../../../../utils'
	import { Button, ClearableInput } from '../../../common'
	import Popover from '../../../Popover.svelte'
	import type { AppViewerContext, ComponentCssProperty } from '../../types'
	import type { AppComponent } from '../component/components'
	import QuickStyleMenu from './QuickStyleMenu.svelte'
	import type { StylePropertyKey } from './quickStyleProperties'

	export let name: string
	export let componentType: AppComponent['type'] | undefined = undefined
	export let value: ComponentCssProperty = {}
	export let forceStyle: boolean = false
	export let forceClass: boolean = false
	export let quickStyleProperties: StylePropertyKey[] | undefined = undefined
	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()
	let isQuickMenuOpen = false

	$: dispatch('change', value)

	function toggleQuickMenu() {
		isQuickMenuOpen = !isQuickMenuOpen
	}

	function applyToAllInstances() {
		if (
			componentType &&
			componentType in ($app?.css || {}) &&
			name in ($app?.css?.[componentType] || {})
		) {
			$app.css![componentType]![name].style = value.style
			sendUserToast(
				`Applied style to all instances of the ${componentType.replace('component', '')} component`
			)
		}
	}
</script>

<div
	class="sticky top-0 z-20 text-lg bg-white font-semibold [font-variant:small-caps] text-gray-700 pt-2 pb-1"
>
	{addWhitespaceBeforeCapitals(name)}
</div>
{#if value}
	<div class="border-l border-gray-400/80 py-1 pl-3.5 ml-0.5">
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
							{#if componentType}
								<Popover placement="bottom" notClickable disapperTimoout={0}>
									<Button
										variant="border"
										color="light"
										size="xs"
										btnClasses="!p-1 !w-[34px] !h-[34px]"
										aria-label="Apply to all instances of this component"
										on:click={applyToAllInstances}
									>
										<Forward size={18} />
									</Button>
									<svelte:fragment slot="text">
										Apply to all instances of this component
									</svelte:fragment>
								</Popover>
							{/if}
							{#if quickStyleProperties?.length}
								<Popover placement="bottom" notClickable disapperTimoout={0}>
									<Button
										variant="border"
										color="light"
										size="xs"
										btnClasses="!p-1 !w-[34px] !h-[34px] {isQuickMenuOpen
											? '!bg-gray-200/60 hover:!bg-gray-200'
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
						<QuickStyleMenu bind:value={value.style} properties={quickStyleProperties} />
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
