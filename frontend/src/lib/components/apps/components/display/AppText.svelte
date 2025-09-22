<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Clipboard } from 'lucide-svelte'
	import { getContext, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { copyToClipboard, isCodeInjection } from '../../../../utils'
	import Button from '../../../common/button/Button.svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { initCss } from '../../utils'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'textcomponent'> | undefined
		render: boolean
		editorMode?: boolean
	}

	let {
		id,
		componentInput,
		horizontalAlignment = 'left',
		verticalAlignment = undefined,
		configuration,
		initializing = $bindable(undefined),
		customCss = undefined,
		render,
		editorMode = $bindable(false)
	}: Props = $props()

	let resolvedConfig = $state(
		initConfig(components['textcomponent'].initialData.configuration, configuration)
	)

	function onEditorMode() {
		autosize()
		setTimeout(() => autosize(), 50)
	}
	const { app, worldStore, mode, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let css = $state(initCss($app.css?.textcomponent, customCss))

	let result: string | undefined = $state(undefined)

	if (
		componentInput?.type == 'template' ||
		(componentInput?.type == 'templatev2' && !isCodeInjection(componentInput.eval))
	) {
		result = componentInput.eval
		initializing = false
	}

	$componentControl[id] = {
		...$componentControl[id],
		setValue(value: string) {
			result = value
		}
	}

	const outputs = initOutput($worldStore, id, {
		result: untrack(() => result),
		loading: initializing
	})

	function getComponent() {
		switch (resolvedConfig.style) {
			case 'Title':
				return 'h1'
			case 'Subtitle':
				return 'h3'
			case 'Body':
				return 'p'
			case 'Caption':
				return 'p'
			case 'Label':
				return 'label'
			default:
				return 'p'
		}
	}

	function getClasses() {
		switch (resolvedConfig.style) {
			case 'Caption':
				return 'text-sm italic text-tertiary'
			case 'Label':
				return 'font-semibold text-sm'
			default:
				return ''
		}
	}

	function getClassesByType() {
		switch (resolvedConfig.style) {
			case 'Title':
				return 'h1-textarea'
			case 'Subtitle':
				return 'h3-textarea'
			case 'Body':
				return 'p-textarea'
			case 'Caption':
				return 'p-textarea'
			case 'Label':
				return 'label'
			default:
				return 'p-textarea'
		}
	}

	let component = $state('p')
	let classes = $state('')

	let rows = 1

	function onInput(e: Event) {
		const target = e.target as HTMLTextAreaElement

		if (target.value != undefined) {
			$componentControl[id]?.setCode?.(target.value)
			autosize()
		}
	}

	function autosize() {
		const el = document.getElementById(`text-${id}`)
		setTimeout(() => {
			if (el !== null) {
				el.style.cssText = 'height:auto; padding:0'
				el.style.cssText = 'height:' + el.scrollHeight + 'px'
			}
			// console.log(el, el?.scrollHeight)
		}, 0)
	}
	$effect(() => {
		editorMode && onEditorMode()
	})
	$effect(() => {
		resolvedConfig.style && (component = getComponent())
	})
	$effect(() => {
		resolvedConfig.style && (classes = getClasses())
	})
	let initialValue = $derived(
		componentInput?.type == 'template' || componentInput?.type == 'templatev2'
			? componentInput.eval
			: ''
	)
	let editableValue = $derived(initialValue ?? '')
</script>

{#each Object.keys(components['textcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.textcomponent}
	/>
{/each}

<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class={twMerge('h-full w-full overflow-hidden', css.container?.class, 'wm-text-container')}
		style={css?.container?.style}
		ondblclick={() => {
			if (!editorMode) {
				editorMode = true
				document.getElementById(`text-${id}`)?.focus()
			}
		}}
		onkeydown={stopPropagation(bubble('keydown'))}
	>
		{#if $mode == 'dnd' && editorMode && (componentInput?.type == 'template' || componentInput?.type == 'templatev2')}
			<AlignWrapper {verticalAlignment}>
				<textarea
					class={twMerge(
						'whitespace-pre-wrap !outline-none !border-0 !bg-transparent !resize-none !ring-0 !p-0',
						css?.text?.class,
						'wm-text',
						classes,
						getClasses(),
						getClassesByType(),
						horizontalAlignment === 'center'
							? 'text-center'
							: horizontalAlignment === 'right'
								? 'text-right'
								: 'text-left'
					)}
					onpointerdown={stopPropagation(bubble('pointerdown'))}
					style={css?.text?.style}
					id={`text-${id}`}
					onpointerenter={() => {
						const elem = document.getElementById(`text-${id}`)
						if (elem) {
							elem.focus()
						}
					}}
					{rows}
					oninput={onInput}
					value={editableValue}
				></textarea>
			</AlignWrapper>
		{:else}
			<AlignWrapper {verticalAlignment}>
				{#if !result || result === ''}
					<div
						class="text-ternary bg-surface-primary flex justify-center items-center h-full w-full"
					>
						{#if resolvedConfig?.disableNoText === false}
							No text
						{/if}
					</div>
				{:else}
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<div
						class="flex flex-wrap gap-0.5 pb-0.5 w-full {$mode === 'dnd' &&
						(componentInput?.type == 'template' || componentInput?.type == 'templatev2')
							? 'cursor-text'
							: 'overflow-auto'}"
					>
						<svelte:element
							this={component}
							class={twMerge(
								'whitespace-pre-wrap grow',

								css?.text?.class,
								classes,
								horizontalAlignment === 'center'
									? 'text-center'
									: horizontalAlignment === 'right'
										? 'text-right'
										: 'text-left'
							)}
							style={css?.text?.style}
						>
							{String(result)}
							{#if resolvedConfig.tooltip && resolvedConfig.tooltip != ''}
								<Tooltip>{resolvedConfig.tooltip}</Tooltip>
							{/if}
						</svelte:element>

						{#if resolvedConfig.copyButton && result}
							<div class="flex items-center">
								<Button
									title="Copy to clipboard"
									variant="border"
									size="xs"
									color="light"
									btnClasses="!p-1"
									on:click={() => copyToClipboard(result)}
								>
									<Clipboard size={14} strokeWidth={2} />
								</Button>
							</div>
						{/if}
					</div>
				{/if}
			</AlignWrapper>
		{/if}
	</div>
</RunnableWrapper>

<style>
	h1 {
		font-size: 29px;
	}
</style>
