<script lang="ts">
	import { Code, Copy, MoreVertical, MoveLeft, MoveRight, Paintbrush2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { slide } from 'svelte/transition'
	import {
		addWhitespaceBeforeCapitals,
		classNames,
		copyToClipboard,
		sendUserToast
	} from '../../../../utils'
	import { Button, ClearableInput } from '../../../common'
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
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

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

	let richEditorOpen = false

	let dynamicClass: boolean = value?.evalClass !== undefined
	let render = 0
</script>

{#key render}
	<div class="flex justify-between items-center p-2 text-xs leading-6 font-bold w-full">
		<div class="capitalize">
			{addWhitespaceBeforeCapitals(name)}
		</div>
		<div>
			<ButtonDropdown hasPadding={false}>
				<svelte:fragment slot="buttonReplacement">
					<Button nonCaptureEvent size="xs" color="light">
						<div class="flex flex-row items-center w-min">
							<MoreVertical size={14} />
						</div>
					</Button>
				</svelte:fragment>
				<svelte:fragment slot="items">
					{#if shouldDisplayLeft}
						<MenuItem
							on:click={() => {
								dispatch('left')
							}}
						>
							<div
								class={classNames(
									'text-primary flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
								)}
							>
								<svelte:component this={MoveLeft} size={14} />
								Copy for this component
							</div>
						</MenuItem>
					{/if}
					{#if shouldDisplayRight}
						<MenuItem
							on:click={() => {
								dispatch('right')
							}}
						>
							<div
								class={classNames(
									'text-primary flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
								)}
							>
								<svelte:component this={MoveRight} size={14} />
								Copy for every {componentType ? ccomponents[componentType].name : 'component'}
							</div>
						</MenuItem>
					{/if}
					{#if wmClass}
						<MenuItem
							on:click={() => {
								copyToClipboard(wmClass)
							}}
						>
							<div
								class={classNames(
									'text-primary flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
								)}
							>
								<svelte:component this={Copy} size={14} />
								Copy {wmClass}
							</div>
						</MenuItem>
					{/if}
				</svelte:fragment>
			</ButtonDropdown>
		</div>
	</div>

	{#if value}
		<div class="p-2 flex flex-col gap-2">
			{#if tooltip}
				<div class="text-tertiary text-2xs py-2">{tooltip}</div>
			{/if}

			{#if value.style !== undefined || forceStyle}
				<div class="pb-2">
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<div class="block w-full">
						<div class="flex flex-row justify-between items-center w-full h-8 mb-1">
							<div class="text-xs font-medium text-tertiary"> Plain CSS </div>

							<div class="flex flex-row gap-1">
								{#if overriden}
									<Badge color="red" small>Overriden by local</Badge>
								{:else if overridding}
									<Badge color="blue" small>Overriding global</Badge>
								{/if}
								{#if quickStyleProperties?.length}
									<ToggleButtonGroup
										bind:selected={richEditorOpen}
										on:selected={() => {
											if (richEditorOpen !== isQuickMenuOpen) {
												toggleQuickMenu()
												richEditorOpen = isQuickMenuOpen
											}
										}}
									>
										<ToggleButton
											small
											light
											value={false}
											icon={Code}
											tooltip="Edit the CSS directly"
										/>
										<ToggleButton
											small
											light
											value={true}
											icon={Paintbrush2}
											tooltip="Open the rich editor to style the component with a visual interface"
										/>
									</ToggleButtonGroup>
								{/if}
							</div>
						</div>

						<ClearableInput
							bind:value={value.style}
							type="textarea"
							disabled={isQuickMenuOpen}
							wrapperClass="h-full min-h-[72px]"
							inputClass="h-full !text-xs !rounded-none !p-2 !shadow-none !border-gray-200 dark:!border-gray-600 "
						/>
					</div>
					{#if quickStyleProperties?.length && isQuickMenuOpen}
						<div class="text-sm mb-1 font-medium">Styling menu</div>
						<div transition:slide|local={{ duration: 200 }} class="w-full border">
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
					<div class="text-xs font-medium text-tertiary mb-1">
						Tailwind classes
						<Tooltip light documentationLink="https://tailwindcss.com/">
							Use any tailwind classes to style your component
						</Tooltip>
					</div>
					<div class="relative">
						<SimpleEditor
							class="h-24 border !rounded-none"
							lang="tailwindcss"
							bind:code={value.class}
							fixedOverflowWidgets={true}
							small
							automaticLayout
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
			<div class="flex flex-row justify-between items-center">
				<div class="text-xs"> Use dynamic class </div>
				<Toggle
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
			</div>
			{#if value?.evalClass && dynamicClass}
				<CssEval key={name} bind:evalClass={value.evalClass} />
			{/if}
		</div>
	{/if}
{/key}
