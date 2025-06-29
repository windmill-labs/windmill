<script lang="ts">
	import { stopPropagation, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { getContext, onDestroy, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput, selectId } from '../../editor/appUtils'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { loadIcon } from '../icon'

	interface Props {
		id: string
		configuration: RichConfigurations
		inputType?: string
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		customCss?: ComponentCustomCSS<'textinputcomponent'> | undefined
		appCssKey?:
			| 'textinputcomponent'
			| 'passwordinputcomponent'
			| 'emailinputcomponent'
			| 'textareainputcomponent'
		render: boolean
		onChange?: string[] | undefined
	}

	let {
		id,
		configuration,
		inputType = 'text',
		verticalAlignment = undefined,
		customCss = undefined,
		appCssKey = 'textinputcomponent',
		render,
		onChange = undefined
	}: Props = $props()

	const {
		app,
		worldStore,
		selectedComponent,
		connectingInput,
		componentControl,
		runnableComponents
	} = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(
		initConfig(components['textinputcomponent'].initialData.configuration, configuration)
	)

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	let initValue = outputs?.result.peak()
	let value: string | undefined = $state(
		!iterContext && initValue && initValue != '' ? initValue : resolvedConfig.defaultValue
	)

	onDestroy(() => {
		listInputs?.remove(id)
	})

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
			outputs?.result.set(value)
		}
	}

	let initialHandleDefault = true

	function onValueChange() {
		let val = value ?? ''
		outputs?.result.set(val)
		if (iterContext && listInputs) {
			listInputs.set(id, val)
		}
		fireOnChange()
	}

	function fireOnChange() {
		if (onChange) {
			onChange.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

	function handleDefault(defaultValue: string | undefined) {
		if (initialHandleDefault) {
			initialHandleDefault = false
			if (value != undefined && value != '') {
				return
			}
		}
		value = defaultValue
	}

	let css = $state(initCss($app.css?.[appCssKey], customCss))

	let beforeIconComponent: any = $state()
	let afterIconComponent: any = $state()

	async function handleBeforeIcon() {
		if (resolvedConfig.beforeIcon) {
			beforeIconComponent = await loadIcon(
				resolvedConfig.beforeIcon,
				beforeIconComponent,
				14,
				undefined,
				undefined
			)
		}
	}

	async function handleAfterIcon() {
		if (resolvedConfig.afterIcon) {
			afterIconComponent = await loadIcon(
				resolvedConfig.afterIcon,
				afterIconComponent,
				14,
				undefined,
				undefined
			)
		}
	}
	$effect.pre(() => {
		resolvedConfig.defaultValue
		untrack(() => handleDefault(resolvedConfig.defaultValue))
	})
	$effect.pre(() => {
		value
		untrack(() => onValueChange())
	})
	let classInput = $derived(
		twMerge(
			'windmillapp w-full py-1.5 px-2 text-sm',
			'app-editor-input',
			css?.input?.class ?? '',
			'wm-input',
			'wm-text-input'
		)
	)
	$effect.pre(() => {
		resolvedConfig.beforeIcon && beforeIconComponent && untrack(() => handleBeforeIcon())
	})
	$effect.pre(() => {
		resolvedConfig.afterIcon && afterIconComponent && untrack(() => handleAfterIcon())
	})
</script>

{#each Object.keys(components['textinputcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.textinputcomponent}
	/>
{/each}

<InitializeComponent {id} />
{#if render}
	{#if inputType === 'textarea'}
		<textarea
			class={twMerge(
				classInput,

				beforeIconComponent && 'pl-8',
				afterIconComponent && 'pr-8',

				'h-full'
			)}
			style="resize:none; {css?.input?.style ?? ''}"
			onpointerdown={(e) => {
				e.stopPropagation()
				!$connectingInput.opened && selectId(e, id, selectedComponent, $app)
			}}
			onkeydown={stopPropagation(bubble('keydown'))}
			bind:value
			placeholder={resolvedConfig.placeholder}
			disabled={resolvedConfig.disabled}
		></textarea>
	{:else}
		<AlignWrapper {render} {verticalAlignment}>
			<div class="relative w-full">
				<div class="absolute top-1/2 -translate-y-1/2 left-2">
					{#if resolvedConfig.beforeIcon}
						{#key resolvedConfig.beforeIcon}
							<div class="min-w-4" bind:this={beforeIconComponent}></div>
						{/key}
					{/if}
				</div>

				{#if inputType === 'password'}
					<input
						class={twMerge(
							classInput,
							resolvedConfig.beforeIcon ? '!pl-8' : '',
							resolvedConfig.afterIcon ? '!pr-8' : ''
						)}
						style={css?.input?.style ?? ''}
						onpointerdown={(e) => {
							e.stopPropagation()
							!$connectingInput.opened && selectId(e, id, selectedComponent, $app)
						}}
						onkeydown={stopPropagation(bubble('keydown'))}
						type="password"
						autocomplete="new-password"
						bind:value
						placeholder={resolvedConfig.placeholder}
						disabled={resolvedConfig.disabled}
					/>
				{:else if inputType === 'text'}
					<input
						class={twMerge(
							classInput,
							resolvedConfig.beforeIcon ? '!pl-8' : '',
							resolvedConfig.afterIcon ? '!pr-8' : ''
						)}
						style={css?.input?.style ?? ''}
						onpointerdown={(e) => {
							e.stopPropagation()
							!$connectingInput.opened && selectId(e, id, selectedComponent, $app)
						}}
						onkeydown={stopPropagation(bubble('keydown'))}
						type="text"
						bind:value
						placeholder={resolvedConfig.placeholder}
						disabled={resolvedConfig.disabled}
					/>
				{:else if inputType === 'email'}
					<input
						class={twMerge(
							classInput,
							resolvedConfig.beforeIcon ? '!pl-8' : '',
							resolvedConfig.afterIcon ? '!pr-8' : ''
						)}
						style={css?.input?.style ?? ''}
						onpointerdown={(e) => {
							e.stopPropagation()
							!$connectingInput.opened && selectId(e, id, selectedComponent, $app)
						}}
						onkeydown={stopPropagation(bubble('keydown'))}
						type="email"
						bind:value
						placeholder={resolvedConfig.placeholder}
						disabled={resolvedConfig.disabled}
					/>
				{/if}

				<div class="absolute top-1/2 -translate-y-1/2 right-2">
					{#if resolvedConfig.afterIcon}
						{#key resolvedConfig.afterIcon}
							<div class="min-w-4" bind:this={afterIconComponent}></div>
						{/key}
					{/if}
				</div>
			</div>
		</AlignWrapper>
	{/if}
{/if}
