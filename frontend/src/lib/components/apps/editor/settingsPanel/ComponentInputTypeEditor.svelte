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
		componentInput.type = 'templatev2'
		componentInput['eval'] = componentInput.value
		componentInput['connections'] = [{ componentId: 'ctx', id: 'email' }]
	}

	let clientWidth: number
</script>

{#if componentInput.fieldType !== 'any'}
	<div class="w-full">
		<div class="mx-auto" bind:clientWidth>
			<ToggleButtonGroup
				on:selected={() => {
					onchange?.()
				}}
				bind:selected={componentInput.type}
			>
				{#if componentInput.fieldType === 'template'}
					{#if componentInput.type == 'template'}
						<ToggleButton
							tooltip={`Templated string (use \$\{<output>.x\} )`}
							value="template"
							disabled={disableStatic}
							icon={CurlyBraces}
							label="Template"
						/>
					{:else}
						<ToggleButton
							tooltip={`Templated string (use \$\{<output>.x\} )`}
							value="templatev2"
							disabled={disableStatic}
							icon={CurlyBraces}
							label="Template"
						/>
					{/if}
				{:else}
					<ToggleButton
						label="Static"
						value="static"
						disabled={disableStatic}
						iconOnly={clientWidth < 250}
						icon={Pen}
					/>
				{/if}

				<ToggleButton value="connected" icon={Plug2} iconOnly={clientWidth < 250} label="Connect" />
				<ToggleButton
					value="runnable"
					icon={FunctionSquare}
					iconOnly={clientWidth < 250}
					label="Compute"
				/>
			</ToggleButtonGroup>
		</div>
	</div>
{/if}
