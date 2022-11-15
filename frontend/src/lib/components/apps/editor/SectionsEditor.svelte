<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppSection, EditorMode } from '../types'
	import { dndzone } from 'svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import ComponentsEditor from './ComponentsEditor.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import ComponentEditor from './ComponentEditor.svelte'

	export let sections: AppSection[]
	export let mode: EditorMode = 'width'

	const flipDurationMs = 200
	const { selection, resizing } = getContext<AppEditorContext>('AppEditorContext')

	function handleSort(e) {
		sections = e.detail.items
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
				{#if mode === 'dnd'}
					<ComponentsEditor
						bind:components={section.components}
						columns={section.columns}
						{sectionIndex}
					/>
				{:else if mode === 'width'}
					<div
						class="h-80 w-full rounded-b-sm flex-row gap-4 flex border-2 border-gray-200 bg-white cursor-pointer "
					>
						<Splitpanes>
							{#each section.components as component}
								<Pane bind:size={component.width} minSize={20}>
									<ComponentEditor bind:component selected={false} />
								</Pane>
							{/each}

							{#if section.components.length < section.columns}
								<Pane
									size={100 - section.components.reduce((accu, curr) => accu + curr.width, 0)}
									minSize={20}
									class="gap-2 w-full flex flex-row"
								>
									{#each Array(section.columns - section.components.length) as _}
										<div
											class="border flex justify-center flex-col items-center w-full h-full bg-green-200 bg-opacity-50"
										>
											<div>Empty</div>
										</div>
									{/each}
								</Pane>
							{/if}
						</Splitpanes>
					</div>
				{/if}
			</section>
		{/each}
	</div>
	<section>
		<div class="h-80 hover:bg-blue-100 rounded-md" />
	</section>
</div>
