<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import {
		faAlignCenter,
		faAlignLeft,
		faAlignRight,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { AppComponent } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'

	export let component: AppComponent | undefined

	const dispatch = createEventDispatcher()
</script>

{#if component}
	<div class="flex flex-col w-full divide-y">
		<span class="text-sm border-y w-full py-1 px-2 bg-gray-800 text-white">Component editor</span>
		<PanelSection title="Inputs">
			<div class="text-sm font-bold" />
			{#if component.type === 'runformcomponent' && component.inputs}
				<InputsSpecsEditor bind:inputSpecs={component.inputs} componenId={component.id} />
			{/if}
			{#if component.type === 'displaycomponent' && component.inputs}
				<InputsSpecsEditor bind:inputSpecs={component.inputs} componenId={component.id} />
			{/if}
		</PanelSection>

		<PanelSection title="Content">
			<div class="w-full text-xs font-bold">Title</div>
			<input type="text" class="w-full" bind:value={component.title} />
			<div class="w-full text-xs font-bold">Description</div>
			<textarea type="text" class="w-full" rows="2" bind:value={component.description} />
		</PanelSection>

		<PanelSection title="Alignement">
			<div class="w-full text-xs font-bold">Horizontal alignement</div>

			<ToggleButtonGroup bind:selected={component.horizontalAlignement}>
				<ToggleButton position="left" value="left" size="xs">
					<Icon data={faAlignLeft} />
				</ToggleButton>
				<ToggleButton position="center" value="center" size="xs">
					<Icon data={faAlignCenter} />
				</ToggleButton>
				<ToggleButton position="right" value="right" size="xs">
					<Icon data={faAlignRight} />
				</ToggleButton>
			</ToggleButtonGroup>
			<div class="w-full text-xs font-bold">Vertical alignement</div>

			<ToggleButtonGroup bind:selected={component.verticalAlignement}>
				<ToggleButton position="left" value="left" size="xs">
					<Icon data={faAlignLeft} />
				</ToggleButton>
				<ToggleButton position="center" value="center" size="xs">
					<Icon data={faAlignCenter} />
				</ToggleButton>
				<ToggleButton position="right" value="right" size="xs">
					<Icon data={faAlignRight} />
				</ToggleButton>
			</ToggleButtonGroup>

			<div class="w-full text-xs font-bold">Width (%)</div>
			<input value={Math.round(component.width)} type="number" class="w-full" disabled />
		</PanelSection>

		<PanelSection title="Danger zone">
			<Button
				size="xs"
				color="red"
				variant="border"
				startIcon={{ icon: faTrashAlt }}
				on:click={() => dispatch('remove')}
			>
				Delete component
			</Button>
		</PanelSection>
	</div>
{/if}
