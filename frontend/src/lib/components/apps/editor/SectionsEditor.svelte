<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppSection, EditorMode } from '../types'
	import { dndzone } from 'svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import ComponentsEditor from './ComponentsEditor.svelte'

	export let sections: AppSection[]

	export let mode: EditorMode = 'width'

	const flipDurationMs = 200
	const { selection } = getContext<AppEditorContext>('AppEditorContext')

	function handleSort(e) {
		sections = e.detail.items
	}

	function addEmptySection() {
		sections = [
			...sections,
			{ components: [], columns: 1, id: getNextId(sections.map((s) => s.id)) }
		]
	}

	function removeSection(index: number) {
		sections.splice(index, 1)
		sections = sections
	}
</script>

<div class="dotted-background h-full w-full p-8">
	<div
		class="flex-col flex justify-start gap-8 pt-8 mb-8"
		use:dndzone={{
			items: sections,
			flipDurationMs,
			type: 'section',
			dropTargetStyle: {
				outline: 'dashed blue',
				outlineOffset: '8px'
			}
		}}
		on:consider={handleSort}
		on:finalize={handleSort}
	>
		{#each sections as section, sectionIndex (section.id)}
			<section animate:flip={{ duration: flipDurationMs }}>
				<span class="bg-gray-500 text-white px-2 text-sm py-1 font-bold rounded-t-sm">
					Section {sectionIndex + 1}
					<Badge>{section.id}</Badge>
				</span>

				{#if mode === 'dnd'}
					<ComponentsEditor
						bind:components={section.components}
						columns={section.columns}
						{sectionIndex}
					/>
				{:else if mode === 'width'}
					<div
						class="h-80 rounded-b-sm flex-row gap-4 p-4 flex border-2 border-gray-500 bg-white cursor-pointer "
					>
						s
					</div>
				{/if}
			</section>
		{/each}
	</div>
	<section>
		<span class="bg-blue-500 text-white px-2 text-sm py-1 font-bold rounded-t-sm">
			Empty section
		</span>
		<div class="h-96 border-2 border-blue-200 border-dashed bg-white flex">
			<Button
				btnClasses="m-auto"
				color="dark"
				size="sm"
				startIcon={{ icon: faPlus }}
				on:click={() => addEmptySection()}
			>
				Add
			</Button>
		</div>
	</section>
</div>

<style>
	.dotted-background {
		background-image: radial-gradient(circle at 1px 1px, #ccc 1px, transparent 0);
		background-size: 40px 40px;
		background-position: 20px 20px;
	}
</style>
