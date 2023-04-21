<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { CurlyBraces, FunctionSquare, Pen, Plug2 } from 'lucide-svelte'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	export let componentInput: AppInput
	export let disableStatic: boolean = false

	const { onchange } = getContext<AppViewerContext>('AppViewerContext')

	$: if (componentInput.fieldType == 'template' && componentInput.type == 'static') {
		//@ts-ignore
		componentInput.type = 'template'
		componentInput['eval'] = componentInput.value
	}

	let clientWidth: number
</script>

{#if componentInput.fieldType !== 'any'}
	<div class="w-full">
		<div class="mx-auto" bind:clientWidth>
			<ToggleButtonGroup on:selected={() => onchange?.()} bind:selected={componentInput.type}>
				{#if componentInput.fieldType === 'template'}
					<ToggleButton
						tooltip={`Templated string (use \$\{<output>.x\} )`}
						value="template"
						disabled={disableStatic}
						icon={CurlyBraces}
						label="Template"
					/>
				{:else}
					<ToggleButton
						label="Static"
						value="static"
						disabled={disableStatic}
						iconOnly={clientWidth < 250}
						icon={Pen}
					/>
				{/if}

				<ToggleButton
					tooltip="Connect to an output"
					value="connected"
					icon={Plug2}
					iconOnly={clientWidth < 250}
					label="Connect"
				/>
				<ToggleButton
					tooltip="Compute it with a script/flow"
					value="runnable"
					icon={FunctionSquare}
					iconOnly={clientWidth < 250}
					label="Compute"
				/>
			</ToggleButtonGroup>
		</div>
	</div>
{/if}
