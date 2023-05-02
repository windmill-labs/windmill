<script lang="ts">
	import { getContext } from 'svelte'

	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../editor/appUtils'
	import { classNames } from '$lib/utils'
	import { AlignWrapper, InputValue } from '../helpers'
	import { concatCustomCss } from '../../utils'

	export let id: string
	export let componentInput: AppInput | undefined
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'iconcomponent'> | undefined = undefined

	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let configuration: RichConfigurations

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')
	let result: string[] | undefined = undefined

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let currentIndex: number | undefined = undefined

	$: css = concatCustomCss($app.css?.iconcomponent, customCss)
</script>

<InputValue {id} input={configuration.currentIndex} bind:value={currentIndex} />
<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
	<AlignWrapper
		{render}
		{horizontalAlignment}
		{verticalAlignment}
		class={css?.container?.class ?? ''}
		style={css?.container?.style ?? ''}
	>
		<ol class="relative z-20 flex justify-between text-sm font-medium text-gray-500">
			{#each result ?? [] as step, index}
				<li class="flex items-center gap-2 p-2">
					<span
						class={classNames(
							'h-6 w-6 rounded-full text-center text-[10px]/6 font-bold',
							index === currentIndex ? 'bg-blue-600 text-white' : 'bg-gray-100 '
						)}
						class:font-bold={currentIndex === index}
					>
						{index + 1}
					</span>

					<span class="hidden sm:block">{step}</span>
				</li>
			{/each}
		</ol>
	</AlignWrapper>
</RunnableWrapper>
