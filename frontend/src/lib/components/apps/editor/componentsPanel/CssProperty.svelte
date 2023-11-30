<script lang="ts">
	import { Copy, MoveLeft, MoveRight, Paintbrush2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { addWhitespaceBeforeCapitals, copyToClipboard, sendUserToast } from '../../../../utils'
	import { Button, ClearableInput } from '../../../common'
	import Popover from '../../../Popover.svelte'
	import type { ComponentCssProperty } from '../../types'
	import { ccomponents, type TypedComponent } from '../component'
	import QuickStyleMenu from './QuickStyleMenu.svelte'
	import type { PropertyGroup } from './quickStyleProperties'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import CssEval from './CssEval.svelte'
	import parse from 'style-to-object'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	export let name: string
	export let value: ComponentCssProperty = {}
	export let forceStyle: boolean = false
	export let forceClass: boolean = false
	export let quickStyleProperties: PropertyGroup[] | undefined = undefined
	export let componentType: TypedComponent['type'] | undefined = undefined
	export let tooltip: string | undefined = undefined
	export let shouldDisplayLeft: boolean = false
	export let shouldDisplayRight: boolean = false
	export let overriden: boolean = false
	export let overridding: boolean = false
	export let wmClass: string | undefined = undefined

	const dispatch = createEventDispatcher()
	let isQuickMenuOpen = false

	$: dispatch('change', value)

	function toggleQuickMenu() {
		try {
			if (!value.style) {
				value.style = ''
			}
			parse(value.style)
			isQuickMenuOpen = !isQuickMenuOpen
		} catch {
			sendUserToast('Invalid CSS: Rich editor cannot be toggled', true)
		}
	}

	let dynamicClass: boolean = value?.evalClass !== undefined
	let render = 0
</script>

{#key render}
	<div class=" border-b flex justify-between items-center p-2 text-xs leading-6 font-bold">
		<div class="flex flex-col gap-1 w-full items-start">
			<div class="flex flex-row h-8 items-center justify-between w-full">
				<div class="capitalize">
					{addWhitespaceBeforeCapitals(name)}
				</div>
				{#if shouldDisplayLeft}
					<Button
						color="light"
						size="xs2"
						variant="border"
						on:click={() => {
							dispatch('left')
						}}
					>
						<div class="flex flex-row gap-2 text-2xs items-center">
							<MoveLeft size={14} />
							Copy for this component
						</div>
					</Button>
				{/if}
				{#if shouldDisplayRight}
					<Button
						color="light"
						size="xs2"
						variant="border"
						on:click={() => {
							dispatch('right')
						}}
					>
						<div class="flex flex-row gap-2 text-2xs items-center">
							Copy for every {componentType ? ccomponents[componentType].name : 'component'}
							<MoveRight size={14} />
						</div>
					</Button>
				{/if}
			</div>
			{#if wmClass}
				<Badge small>
					<div class="flex flex-row gap-1 items-center">
						{wmClass}
						<Button
							color="light"
							size="xs2"
							on:click={() => {
								copyToClipboard(wmClass)
							}}
						>
							<Copy size={14} />
						</Button>
					</div>
				</Badge>
			{/if}
		</div>
	</div>

	{#if value}
		<div class="p-2 flex flex-col">
			{#if tooltip}
				<div class="text-tertiary text-2xs py-2">{tooltip}</div>
			{/if}
			{#if value.style !== undefined || forceStyle}
				<div class="pb-2">
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label class="block w-full">
						<div class="flex flex-row justify-between items-center w-full h-8">
							<div class="text-xs font-medium text-tertiary"> Plain CSS </div>
							{#if overriden}
								<Badge color="red" small>Overriden by local</Badge>
							{:else if overridding}
								<Badge color="blue" small>Overriding global</Badge>
							{/if}
						</div>

						<div class="flex gap-1">
							<div class="relative grow">
								<ClearableInput
									bind:value={value.style}
									type="textarea"
									wrapperClass="h-full min-h-[72px]"
									inputClass="h-full !text-xs  !rounded-none !p-2"
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
					{#if componentType && ccomponents?.[componentType]?.quickstyle?.[name]?.quickCss}
						<div class="flex flex-row gap-1 items-center mt-1 flex-wrap">
							{#each ccomponents?.[componentType]?.quickstyle?.[name].quickCss ?? [] as v}
								<Badge
									small
									baseClass="cursor-pointer"
									on:click={() => {
										value.style = value.style === '' ? `${v};` : `${value.style} ${v};`
									}}
								>
									{v}
								</Badge>
							{/each}
						</div>
					{/if}
				</div>
			{/if}

			{#if value.class !== undefined || forceClass}
				<!-- svelte-ignore a11y-label-has-associated-control -->
				<label class="block">
					<div class="text-xs font-medium text-tertiary">
						Tailwind classes
						<Tooltip light documentationLink="https://tailwindcss.com/">
							Use any tailwind classes to style your component
						</Tooltip>
					</div>
					<div class="relative">
						<SimpleEditor
							class="h-24"
							lang="tailwindcss"
							bind:code={value.class}
							fixedOverflowWidgets={true}
							small
							automaticLayout
							deno={false}
							subType="tailwind"
						/>
					</div>
					{#if componentType && ccomponents?.[componentType]?.quickstyle?.[name]?.quickTailwindClasses}
						<div class="flex flex-row gap-1 items-center mt-1 flex-wrap">
							{#each ccomponents?.[componentType]?.quickstyle?.[name]?.quickTailwindClasses ?? [] as cls}
								<Badge
									baseClass="cursor-pointer"
									small
									on:click={() => {
										value.class = value.class === '' ? cls : `${value.class} ${cls}`
										render++
									}}
								>
									{cls}
								</Badge>
							{/each}
						</div>
					{/if}
				</label>
			{/if}

			<Toggle
				options={{
					right: 'Use dynamic class'
				}}
				size="xs"
				bind:checked={dynamicClass}
				on:change={(e) => {
					if (e.detail && !value.evalClass) {
						value.evalClass = {
							type: 'evalv2',
							expr: '',
							connections: [],
							fieldType: 'text'
						}
					} else {
						value.evalClass = undefined
					}
				}}
			/>

			{#if value?.evalClass && dynamicClass}
				<CssEval key={name} bind:evalClass={value.evalClass} />
			{/if}
		</div>
	{/if}
{/key}
