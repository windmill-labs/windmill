<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import PickScript from '$lib/components/flows/pickers/PickScript.svelte'
	import {
		faAlignCenter,
		faAlignLeft,
		faAlignRight,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { AppComponent, AppEditorContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import ComponentInputsSpecsEditor from './ComponentInputsSpecsEditor.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import PickFlow from './PickFlow.svelte'

	export let component: AppComponent | undefined

	const { app } = getContext<AppEditorContext>('AppEditorContext')
	const dispatch = createEventDispatcher()

	function setTriggerable(path: string, type: 'flow' | 'script') {
		if (component?.id) {
			$app.policy.triggerables[component.id] = {
				type,
				path
			}
		}
	}
</script>

{#if component}
	{#key component.id}
		<div class="flex flex-col w-full divide-y">
			<span class="text-sm border-y w-full py-1 px-2 bg-gray-800 text-white">Component editor</span>

			<PanelSection title="Inputs">
				{#if component.type === 'runformcomponent'}
					<InputsSpecsEditor bind:inputSpecs={component.inputs} />
				{/if}

				{#if component.type === 'runformcomponent' && $app.policy.triggerables[component.id] === undefined}
					<span class="text-sm">Select a script or a flow to continue</span>
					<PickScript
						kind="script"
						on:pick={({ detail }) => {
							setTriggerable(detail.path, 'script')
						}}
					/>
					<PickFlow
						on:pick={({ detail }) => {
							setTriggerable(detail.path, 'flow')
						}}
					/>
				{/if}

				{#if component.type === 'displaycomponent' && component.componentInputs}
					<ComponentInputsSpecsEditor bind:componentInputs={component.componentInputs} />
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
	{/key}
{/if}
