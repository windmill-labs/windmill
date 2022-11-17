<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import BarChartComponent from '../components/charts/BarChartComponent.svelte'
	import DisplayComponent from '../components/DisplayComponent.svelte'
	import RunFormComponent from '../components/RunFormComponent.svelte'
	import TableComponent from '../components/TableComponent.svelte'
	import TextComponent from '../components/common/TextComponent.svelte'
	import type { AppComponent, AppEditorContext } from '../types'
	import { displayData } from '../utils'

	export let component: AppComponent
	export let selected: boolean

	const { staticOutputs, connectingInput, mode } = getContext<AppEditorContext>('AppEditorContext')

	function connectInput(output: string) {
		if ($connectingInput) {
			$connectingInput = {
				opened: false,
				input: {
					id: component.id,
					name: output,
					type: 'output',
					defaultValue: undefined
				}
			}
		}
	}
</script>

{#if component}
	<div class="h-full flex flex-col w-full relative">
		{#if selected}
			<span
				class={classNames(
					'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute -top-5',
					selected ? 'bg-indigo-500' : 'bg-gray-500'
				)}
			>
				{displayData[component.type].name} - {component.id}
			</span>
		{/if}

		<div
			class={classNames(
				'p-2 border overflow-auto cursor-pointer  h-full bg-white',
				selected ? 'border-blue-500' : 'border-white',
				$mode === 'preview' ? 'border-white' : 'hover:border-blue-500 '
			)}
		>
			{#if $connectingInput.opened && $staticOutputs[component.id]}
				<div
					class="absolute top-0 bottom-0 left-0 right-0 bg-opacity-80 w-full h-full bg-gray-500 flex justify-center items-center flex-col gap-2"
				>
					{#each $staticOutputs[component.id] as output}
						<Button color="dark" on:click={() => connectInput(output)}>
							{output}
						</Button>
					{/each}
				</div>
			{/if}

			{#if component.type === 'runformcomponent'}
				<RunFormComponent
					{...component}
					bind:inputs={component.inputs}
					bind:staticOutputs={$staticOutputs[component.id]}
				/>
			{:else if component.type === 'displaycomponent'}
				<DisplayComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{:else if component.type === 'barchartcomponent'}
				<BarChartComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{:else if component.type === 'tablecomponent'}
				<TableComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{:else if component.type === 'textcomponent'}
				<TextComponent
					{...component}
					bind:staticOutputs={$staticOutputs[component.id]}
					bind:inputs={component.inputs}
				/>
			{/if}
		</div>
	</div>
{/if}
