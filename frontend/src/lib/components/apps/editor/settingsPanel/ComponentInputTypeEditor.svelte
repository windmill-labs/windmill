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
	<div class="w-full overflow-x-auto" bind:clientWidth>
		<ToggleButtonGroup on:selected={() => onchange?.()} bind:selected={componentInput.type}>
			{#if componentInput.fieldType === 'template'}
				<ToggleButton position="left" value="template" size="xs" disable={disableStatic}>
					{brackets}&nbsp;<span class="hidden lg:block">Template</span>
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
						<span class="hidden lg:block">Static</span>
					{/if}
				</ToggleButton>
			{/if}

			<ToggleButton
				title="Connect"
				value="connected"
				position="center"
				startIcon={{ icon: faArrowRight }}
				size="xs"
			>
				{#if clientWidth > 250}
					<span class="hidden lg:block">Connect</span>
				{/if}
			</ToggleButton>
			<ToggleButton
				title="Compute"
				position="right"
				value="runnable"
				startIcon={{ icon: faCode }}
				size="xs"
			>
				{#if clientWidth > 250}
					<span class="hidden lg:block">Compute</span>
				{/if}
			</ToggleButton>
		</ToggleButtonGroup>
	</div>
{/if}
