<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { faArrowRight, faCode, faPen } from '@fortawesome/free-solid-svg-icons'
	import type { AppInput } from '../../inputType'

	export let componentInput: AppInput
	export let disableStatic: boolean = false
</script>

{#if componentInput.fieldType !== 'any'}
	<div class="w-full">
		<ToggleButtonGroup bind:selected={componentInput.type}>
			<ToggleButton
				position="left"
				value="static"
				startIcon={{ icon: faPen }}
				size="xs"
				disable={disableStatic}
			>
				Static
			</ToggleButton>
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
