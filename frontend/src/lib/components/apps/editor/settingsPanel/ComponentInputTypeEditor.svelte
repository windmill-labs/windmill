<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { faArrowRight, faCode, faPen } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'

	export let componentInput: AppInput
	export let disableStatic: boolean = false

	const { onchange } = getContext<AppViewerContext>('AppViewerContext')

	$: if (componentInput.fieldType == 'template' && componentInput.type == 'static') {
		//@ts-ignore
		componentInput.type = 'template'
		componentInput['eval'] = componentInput.value
	}

	const brackets = '${}'

	let clientWidth: number
</script>

{#if componentInput.fieldType !== 'any'}
	<div class="w-full">
		<div class="mx-auto" bind:clientWidth>
			<ToggleButtonGroup on:selected={() => onchange?.()} bind:selected={componentInput.type}>
				{#if componentInput.fieldType === 'template'}
					<ToggleButton
						title={`Templated string (use \$\{<output>.x\} )`}
						position="left"
						value="template"
						size="xs"
						disable={disableStatic}
					>
						<span class="font-mono text-2xs h-3 -mt-0.5">
							{brackets}
						</span>
						{#if clientWidth > 250}
							&nbsp;Template
						{/if}
					</ToggleButton>
				{:else}
					<ToggleButton
						title="Static"
						position="left"
						value="static"
						startIcon={{ icon: faPen }}
						size="xs"
						disable={disableStatic}
					>
						{#if clientWidth > 250}
							Static
						{/if}
					</ToggleButton>
				{/if}

				<ToggleButton
					title="Connect to an output"
					value="connected"
					position="center"
					startIcon={{ icon: faArrowRight }}
					size="xs"
				>
					{#if clientWidth > 250}
						Connect
					{/if}
				</ToggleButton>
				<ToggleButton
					title="Compute it with a script/flow"
					position="right"
					value="runnable"
					startIcon={{ icon: faCode }}
					size="xs"
				>
					{#if clientWidth > 250}
						Compute
					{/if}
				</ToggleButton>
			</ToggleButtonGroup>
		</div>
	</div>
{/if}
