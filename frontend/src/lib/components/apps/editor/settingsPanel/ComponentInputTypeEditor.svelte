<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { faArrowRight, faCode, faPen } from '@fortawesome/free-solid-svg-icons'
	import type { AppInput } from '../../inputType'

	export let componentInput: AppInput
	export let disableStatic: boolean = false

	$: if (componentInput.fieldType == 'textarea' && componentInput.type == 'static') {
		//@ts-ignore
		componentInput.type = 'template'
	}

	const brackets = '${}'
</script>

{#if componentInput.fieldType !== 'any'}
	<div class="w-full overflow-x-auto">
		<ToggleButtonGroup bind:selected={componentInput.type}>
			{#if componentInput.fieldType === 'textarea'}
				<ToggleButton position="left" value="template" size="xs" disable={disableStatic}>
					{brackets} Templatable
				</ToggleButton>
			{:else}
				<ToggleButton
					position="left"
					value="static"
					startIcon={{ icon: faPen }}
					size="xs"
					disable={disableStatic}
				>
					Static
				</ToggleButton>
			{/if}

			<ToggleButton
				value="connected"
				position="center"
				startIcon={{ icon: faArrowRight }}
				size="xs"
			>
				Connected
			</ToggleButton>
			<ToggleButton position="right" value="runnable" startIcon={{ icon: faCode }} size="xs">
				Computed
			</ToggleButton>
		</ToggleButtonGroup>
	</div>
{/if}
