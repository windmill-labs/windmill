<script lang="ts">
	import { Code, Copy, MoveLeft, MoveRight, Paintbrush2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { slide } from 'svelte/transition'
	import { addWhitespaceBeforeCapitals, copyToClipboard, sendUserToast } from '../../../../utils'
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
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { tailwindClasses } from './tailwindUtils'
	import { deepEqual } from 'fast-equals'

	interface Props {
		name: string
		value?: ComponentCssProperty
		forceStyle?: boolean
		forceClass?: boolean
		quickStyleProperties?: PropertyGroup[] | undefined
		componentType?: TypedComponent['type'] | undefined
		tooltip?: string | undefined
		shouldDisplayLeft?: boolean
		shouldDisplayRight?: boolean
		overriden?: boolean
		overridding?: boolean
		wmClass?: string | undefined
	}

	let {
		name,
		value = $bindable(),
		forceStyle = false,
		forceClass = false,
		quickStyleProperties = undefined,
		componentType = undefined,
		tooltip = undefined,
		shouldDisplayLeft = false,
		shouldDisplayRight = false,
		overriden = false,
		overridding = false,
		wmClass = undefined
	}: Props = $props()

	const dispatch = createEventDispatcher()
	let isQuickMenuOpen = $state(false)

	let prevValue = structuredClone($state.snapshot(value))
	$effect(() => {
		if (deepEqual(prevValue, value)) return
		prevValue = structuredClone($state.snapshot(value))
		dispatch('change', value)
	})

	function toggleQuickMenu() {
		if (!value) return
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

	let richEditorOpen = $state(false)

	let dynamicClass: boolean = $state(value?.evalClass !== undefined)
	let render = $state(0)
</script>

{#key render}
	<div class="flex justify-between items-center p-2 text-xs leading-6 font-bold w-full">
		<div class="capitalize">
			{addWhitespaceBeforeCapitals(name)}
		</div>
		<div class="flex flex-row items-center gap-1">
			{#if shouldDisplayLeft}
				<Popover placement="bottom" notClickable disappearTimeout={0}>
					<Button
						color="light"
						size="xs2"
						iconOnly
						startIcon={{ icon: MoveLeft }}
						on:click={() => dispatch('left')}
					/>
					{#snippet text()}
						{'Copy for this component'}
					{/snippet}
				</Popover>
			{/if}
			{#if shouldDisplayRight}
				<Popover placement="bottom" notClickable disappearTimeout={0}>
					<Button
						color="light"
						size="xs2"
						iconOnly
						startIcon={{ icon: MoveRight }}
						on:click={() => dispatch('right')}
					/>
					{#snippet text()}
						Copy for every {componentType ? ccomponents[componentType].name : 'component'}
					{/snippet}
				</Popover>
			{/if}
			<Popover placement="bottom" notClickable disappearTimeout={0}>
				<Button
					color="light"
					size="xs2"
					iconOnly
					startIcon={{ icon: Copy }}
					on:click={() => copyToClipboard(wmClass)}
				/>
				{#snippet text()}
					Copy {wmClass}
				{/snippet}
			</Popover>
		</div>
	</div>

	{#if value}
		<div class="p-2 flex flex-col gap-2">
			{#if tooltip}
				<div class="text-primary text-2xs py-2">{tooltip}</div>
			{/if}

			{#if value.style !== undefined || forceStyle}
				<div class="pb-2">
					<!-- svelte-ignore a11y_label_has_associated_control -->
					<div class="block w-full">
						<div class="flex flex-row justify-between items-center w-full h-8 mb-1">
							<div class="text-xs font-medium text-primary"> Plain CSS </div>

							<div class="flex flex-row gap-1">
								{#if overriden}
									<Badge color="red" small>Overriden by local</Badge>
								{:else if overridding}
									<Badge color="blue" small>Overriding global</Badge>
								{/if}
								{#if quickStyleProperties?.length}
									<ToggleButtonGroup
										selected={richEditorOpen ? 'true' : 'false'}
										on:selected={({ detail }) => {
											richEditorOpen = detail === 'true'
											if (richEditorOpen !== isQuickMenuOpen) {
												toggleQuickMenu()
												richEditorOpen = isQuickMenuOpen
											}
										}}
									>
										{#snippet children({ item })}
											<ToggleButton
												small
												value={'false'}
												icon={Code}
												tooltip="Edit the CSS directly"
												{item}
											/>
											<ToggleButton
												small
												value={'true'}
												icon={Paintbrush2}
												tooltip="Open the rich editor to style the component with a visual interface"
												{item}
											/>
										{/snippet}
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
						<div class="text-xs mb-1 font-medium">Rich editor</div>
						<div transition:slide|local={{ duration: 200 }} class="w-full">
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
									clickable
									onclick={() => {
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
				<!-- svelte-ignore a11y_label_has_associated_control -->
				<div class="text-xs font-medium text-primary mb-1">
					Tailwind classes
					<Tooltip light documentationLink="https://tailwindcss.com/">
						Use any tailwind classes to style your component
					</Tooltip>
				</div>
				<div class="relative">
					<SimpleEditor
						class="h-24 border !rounded-none"
						lang="tailwindcss"
						{tailwindClasses}
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
								clickable
								onclick={() => {
									value.class = value.class === '' ? cls : `${value.class} ${cls}`
									render++
								}}
							>
								{cls}
							</Badge>
						{/each}
					</div>
				{/if}
			{/if}
			<div class="flex flex-row justify-between items-center">
				<div class="text-xs flex flex-row items-center justify-center">
					Use dynamic class
					<Tooltip light>
						Eval an expression that return a list of class as string to dynamically add classes to
						the component. The styling can then be dynamic using the global CSS Editor.
					</Tooltip>
				</div>
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
