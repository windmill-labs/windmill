<script lang="ts">
	import { Clipboard } from 'lucide-svelte'
	import { copyToClipboard } from '../../../../utils'
	import Button from '../../../common/button/Button.svelte'
	import Popover from '../../../Popover.svelte'
	import type { AppInput } from '../../inputType'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { twMerge } from 'tailwind-merge'
	import type {
		AppEditorContext,
		AppViewerContext,
		ComponentCustomCSS,
		RichConfigurations
	} from '../../types'
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'textcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, mode } = getContext<AppViewerContext>('AppViewerContext')

	const { ontextfocus } = getContext<AppEditorContext>('AppEditorContext')

	initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: string | undefined = undefined
	let style: 'Title' | 'Subtitle' | 'Body' | 'Caption' | 'Label' | undefined = undefined
	let copyButton: boolean
	let tooltip: string = ''

	function getComponent() {
		switch (style) {
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
		switch (style) {
			case 'Caption':
				return 'text-sm italic text-gray-500'
			case 'Label':
				return 'font-semibold text-sm'
			default:
				return ''
		}
	}

	let component = 'p'
	let classes = ''
	$: style && (component = getComponent())
	$: style && (classes = getClasses())
</script>

<InputValue {id} input={configuration.style} bind:value={style} />
<InputValue {id} input={configuration.copyButton} bind:value={copyButton} />
<InputValue {id} input={configuration.tooltip} bind:value={tooltip} />

<RunnableWrapper {render} {componentInput} {id} bind:initializing bind:result>
	<div class="h-full w-full overflow-hidden">
		<AlignWrapper {horizontalAlignment} {verticalAlignment}>
			{#if !result || result === ''}
				<div class="text-gray-400 bg-gray-100 flex justify-center items-center h-full w-full">
					No text
				</div>
			{:else}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<div
					class="flex flex-wrap gap-2 pb-0.5  {$mode === 'dnd' && componentInput?.type == 'template'
						? 'cursor-text'
						: ''}"
					on:click={() => {
						if ($mode === 'dnd' && componentInput?.type == 'template') {
							$ontextfocus?.()
						}
					}}
				>
					<svelte:element
						this={component}
						class={twMerge(
							'whitespace-pre-wrap',
							$app.css?.['textcomponent']?.['text']?.class,
							customCss?.text?.class,
							classes
						)}
						style={[$app.css?.['textcomponent']?.['text']?.style, customCss?.text?.style].join(';')}
					>
						{String(result)}
						{#if tooltip != ''}
							<Tooltip>{tooltip}</Tooltip>
						{/if}
						{#if copyButton && result}
							<Popover notClickable>
								<Button
									variant="border"
									size="xs"
									color="dark"
									btnClasses="!p-1"
									on:click={() => copyToClipboard(result)}
								>
									<Clipboard size={14} strokeWidth={2} />
								</Button>
								<svelte:fragment slot="text">Copy to clipboard</svelte:fragment>
							</Popover>
						{/if}
					</svelte:element>
				</div>
			{/if}
		</AlignWrapper>
	</div>
</RunnableWrapper>
