<script lang="ts">
	import { getContext, onDestroy } from 'svelte'
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

	export let id: string
	export let configuration: RichConfigurations
	export let inputType = 'text'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'textinputcomponent'> | undefined = undefined
	export let appCssKey:
		| 'textinputcomponent'
		| 'passwordinputcomponent'
		| 'emailinputcomponent'
		| 'textareainputcomponent' = 'textinputcomponent'
	export let render: boolean

	const { app, worldStore, selectedComponent, connectingInput, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['textinputcomponent'].initialData.configuration,
		configuration
	)

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let value: string | undefined = resolvedConfig.defaultValue

	let outputs = initOutput($worldStore, id, {
		result: ''
	})

	onDestroy(() => {
		listInputs?.remove(id)
	})

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
		}
	}

	$: handleDefault(resolvedConfig.defaultValue)

	$: {
		let val = value ?? ''
		outputs?.result.set(val)
		if (iterContext && listInputs) {
			listInputs.set(id, val)
		}
	}

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue
	}

	let css = initCss($app.css?.[appCssKey], customCss)

	$: classInput = twMerge(
		'windmillapp w-full  py-1.5 text-sm focus:ring-indigo-100 px-2',
		css?.input?.class ?? '',
		resolvedConfig.disabled ? 'placeholder:text-gray-400 dark:placeholder:text-gray-600' : '',
		'wm-text-input'
	)

	let beforeIconComponent: any
	let afterIconComponent: any

	$: resolvedConfig.beforeIcon && handleBeforeIcon()
	$: resolvedConfig.afterIcon && handleAfterIcon()

	async function handleBeforeIcon() {
		if (resolvedConfig.beforeIcon) {
			beforeIconComponent = await loadIcon(resolvedConfig.beforeIcon)
		}
	}

	async function handleAfterIcon() {
		if (resolvedConfig.afterIcon) {
			afterIconComponent = await loadIcon(resolvedConfig.afterIcon)
		}
	}
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
			on:pointerdown|stopPropagation={(e) =>
				!$connectingInput.opened && selectId(e, id, selectedComponent, $app)}
			on:keydown|stopPropagation
			bind:value
			placeholder={resolvedConfig.placeholder}
			disabled={resolvedConfig.disabled}
		/>
	{:else}
		<AlignWrapper {render} {verticalAlignment}>
			<div class="relative w-full">
				<div class="absolute top-1/2 -translate-y-1/2 left-2">
					{#if resolvedConfig.beforeIcon && beforeIconComponent}
						<svelte:component this={beforeIconComponent} size={14} />
					{/if}
				</div>

				{#if inputType === 'password'}
					<input
						class={twMerge(classInput, beforeIconComponent && 'pl-8', afterIconComponent && 'pr-8')}
						style={css?.input?.style ?? ''}
						on:pointerdown|stopPropagation={(e) =>
							!$connectingInput.opened && selectId(e, id, selectedComponent, $app)}
						on:keydown|stopPropagation
						type="password"
						bind:value
						placeholder={resolvedConfig.placeholder}
						disabled={resolvedConfig.disabled}
					/>
				{:else if inputType === 'text'}
					<input
						class={twMerge(classInput, beforeIconComponent && 'pl-8', afterIconComponent && 'pr-8')}
						style={css?.input?.style ?? ''}
						on:pointerdown|stopPropagation={(e) =>
							!$connectingInput.opened && selectId(e, id, selectedComponent, $app)}
						on:keydown|stopPropagation
						type="text"
						bind:value
						placeholder={resolvedConfig.placeholder}
						disabled={resolvedConfig.disabled}
					/>
				{:else if inputType === 'email'}
					<input
						class={twMerge(classInput, beforeIconComponent && 'pl-8', afterIconComponent && 'pr-8')}
						style={css?.input?.style ?? ''}
						on:pointerdown|stopPropagation={(e) =>
							!$connectingInput.opened && selectId(e, id, selectedComponent, $app)}
						on:keydown|stopPropagation
						type="email"
						bind:value
						placeholder={resolvedConfig.placeholder}
						disabled={resolvedConfig.disabled}
					/>
				{/if}

				<div class="absolute top-1/2 -translate-y-1/2 right-2">
					{#if resolvedConfig.afterIcon && afterIconComponent}
						<svelte:component this={afterIconComponent} size={14} />
					{/if}
				</div>
			</div>
		</AlignWrapper>
	{/if}
{/if}
