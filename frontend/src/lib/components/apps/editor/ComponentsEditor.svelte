<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppComponent, AppEditorContext } from '../types'
	import ComponentEditor from './ComponentEditor.svelte'
	import { dndzone } from 'svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import { classNames } from '$lib/utils'

	export let components: AppComponent[]
	export let columns: number
	export let sectionIndex: number

	const flipDurationMs = 200
	const { selection } = getContext<AppEditorContext>('AppEditorContext')

	let mouseInside: boolean = false

	function handleSort(e) {
		components = e.detail.items
	}
</script>

<div
	class="h-80 rounded-b-sm flex-row gap-4 p-4 flex border-2 border-gray-500 bg-white cursor-pointer "
	on:mouseenter={() => {
		mouseInside = true
	}}
	on:mouseleave={() => {
		mouseInside = false
	}}
>
	<div class="dotted-background h-full flex flex-row gap-2 w-full">
		{#if components.length > 0}
			<div
				class={classNames('flex flex-row gap-2 h-full w-full')}
				use:dndzone={{
					items: components,
					flipDurationMs,
					type: 'component',
					dropTargetStyle: {
						outline: 'dashed blue',
						outlineOffset: '2px'
					}
				}}
				on:consider={handleSort}
				on:finalize={handleSort}
			>
				{#each components as component, componentIndex (component.id)}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						on:click={() => {
							$selection = { componentIndex, sectionIndex }
						}}
						animate:flip={{ duration: flipDurationMs }}
						class={`w-[${component.width}%] bg-white`}
					>
						<ComponentEditor bind:component />
					</div>
				{/each}
			</div>
		{/if}

		{#if columns - components.length >= 1}
			<div class="w-auto">
				<div
					class={classNames('h-1/2 flex flex-col w-full')}
					use:dndzone={{
						type: 'component',
						items: [],
						dropFromOthersDisabled: mouseInside,
						dragDisabled: true,
						dropTargetStyle: {
							outline: 'dashed blue',
							outlineOffset: '2px'
						}
					}}
					on:consider={(e) => {}}
					on:finalize={(e) => {}}
				>
					<div
						class="p-2 border-dashed border border-gray-400 cursor-pointer hover:bg-blue-100 h-full flex justify-center items-center text-sm"
					>
						Empty component
					</div>
				</div>
				<div
					class={classNames('h-1/2 flex flex-col w-full')}
					use:dndzone={{
						type: 'new-component',
						items: [],
						dropFromOthersDisabled: false,
						dragDisabled: true,
						dropTargetStyle: {
							outline: 'dashed green',
							outlineOffset: '2px'
						}
					}}
					on:consider={(e) => {}}
					on:finalize={(e) => {}}
				>
					<div
						class="p-2 border-dashed border border-gray-400 cursor-pointer hover:bg-blue-100 h-full flex justify-center items-center text-sm"
					>
						New component
					</div>
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.dotted-background {
		background-image: radial-gradient(circle at 1px 1px, #ccc 1px, transparent 0);
		background-size: 40px 40px;
		background-position: 20px 20px;
	}
</style>
