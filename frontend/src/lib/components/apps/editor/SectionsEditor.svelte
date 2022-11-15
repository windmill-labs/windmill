<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppComponent, AppEditorContext, AppSection, EditorMode } from '../types'
	import { dndzone } from 'svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import ComponentsEditor from './ComponentsEditor.svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'

	export let sections: AppSection[]
	export let mode: EditorMode = 'width'
	const flipDurationMs = 200
	const { selection, resizing, app } = getContext<AppEditorContext>('AppEditorContext')
	let emptyComponents: AppComponent[] = []

	let dropped = false
	function handleSort(e) {
		sections = e.detail.items
	}

	$: emptyComponents.length > 0 && dropped && handleCreateSection()

	function handleCreateSection() {
		const component = emptyComponents[0]

		component.id = getNextId(
			$app.sections
				.map((s) => s.components)
				.flat()
				.map((c) => c.id)
		)

		component.width = 100

		sections = [
			...sections,
			{
				components: [component],
				columns: 3,
				id: getNextId(sections.map((s) => s.id)),
				title: 'New section',
				description: 'section'
			}
		]

		emptyComponents = []
		dropped = false
	}

	function removeEmptySections() {
		sections = sections.filter((section) => section.components.length > 0)
	}

	$: sections && removeEmptySections()
</script>

<div class="w-full p-4 relative">
	<div
		class="flex-col flex gap-4 mb-4 w-full rounded-md"
		use:dndzone={{
			items: sections,
			flipDurationMs,
			type: 'section',
			dragDisabled: mode === 'width' || $resizing,
			dropTargetStyle: {
				outline: '1px dashed blue',
				outlineOffset: '8px'
			}
		}}
		on:consider={handleSort}
		on:finalize={handleSort}
	>
		{#each sections as section, sectionIndex (section.id)}
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<section
				animate:flip={{ duration: flipDurationMs }}
				on:click={() => {
					$selection = { sectionIndex, componentIndex: undefined }
				}}
			>
				<ComponentsEditor
					bind:components={section.components}
					columns={section.columns}
					{sectionIndex}
				/>
			</section>
		{/each}
	</div>
	{#if mode !== 'preview'}
		<section>
			<div
				class="h-80 hover:bg-blue-100 rounded-md"
				use:dndzone={{
					items: emptyComponents,
					flipDurationMs,
					type: 'component',
					dropTargetStyle: {
						outline: '1px dashed blue',
						outlineOffset: '8px'
					}
				}}
				on:consider={(e) => {
					emptyComponents = e.detail.items
				}}
				on:finalize={(e) => {
					emptyComponents = e.detail.items
					dropped = true
				}}
			>
				{#each emptyComponents as emptyComponnet, emptyIndex (emptyComponnet.id)}
					<div>{emptyIndex}</div>
				{/each}
			</div>
		</section>
	{/if}
</div>
