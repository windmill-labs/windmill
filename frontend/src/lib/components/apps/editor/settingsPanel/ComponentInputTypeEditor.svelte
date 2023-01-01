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

	let clientWidth: number
</script>

{#if componentInput.fieldType !== 'any'}
	<div class="w-full overflow-x-auto" bind:clientWidth>
		<ToggleButtonGroup bind:selected={componentInput.type}>
			{#if componentInput.fieldType === 'textarea'}
				<ToggleButton position="left" value="template" size="xs" disable={disableStatic}>
					{brackets}&nbsp;<span class="hidden lg:block">Template</span>
				</ToggleButton>
			{:else}
				<ToggleButton
					position="left"
					value="static"
					startIcon={{ icon: faPen }}
					size="xs"
					disable={disableStatic}
				>
					{#if clientWidth > 250}
						<span class="hidden lg:block"> Static </span>
					{/if}
				</ToggleButton>
			{/if}

			<ToggleButton
				value="connected"
				position="center"
				startIcon={{ icon: faArrowRight }}
				size="xs"
			>
				{#if clientWidth > 250}
					<span class="hidden lg:block"> Connected </span>
				{/if}
			</ToggleButton>
			<ToggleButton position="right" value="runnable" startIcon={{ icon: faCode }} size="xs">
				{#if clientWidth > 250}
					<span class="hidden lg:block"> Computed </span>
				{/if}
			</ToggleButton>
		</ToggleButtonGroup>
	</div>
{/if}
