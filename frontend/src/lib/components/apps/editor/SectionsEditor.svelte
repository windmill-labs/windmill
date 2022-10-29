<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'

	import type { AppEditorContext, AppSection } from '../types'
	import ComponentEditor from './ComponentEditor.svelte'

	export let sections: AppSection[]

	function addEmptySection() {
		sections = [...sections, { components: [], columns: 1 }]
	}

	function removeSection(index: number) {
		sections.splice(index, 1)
		sections = sections
	}
	const { selection } = getContext<AppEditorContext>('AppEditorContext')
</script>

<div class="dotted-background h-full w-full px-8">
	<div class="flex-col flex justify-start gap-8 pt-8">
		{#each sections as section, sectionIndex (sectionIndex)}
			<section>
				<span class="bg-gray-500 text-white px-2 text-sm py-1 font-bold rounded-t-sm">
					Section {sectionIndex + 1}
				</span>

				<div
					class="h-96 rounded-b-sm flex-row gap-4  p-4 flex border-2 border-gray-500 bg-white cursor-pointer "
				>
					{#each Array(section.columns) as _, componentIndex (componentIndex)}
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<div
							class="w-full"
							on:click={() => {
								$selection = { sectionIndex, componentIndex }
							}}
						>
							<ComponentEditor bind:component={section.components[componentIndex]} />
						</div>
					{/each}
				</div>
			</section>
		{/each}

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
</div>

<style>
	.dotted-background {
		background-image: radial-gradient(circle at 1px 1px, #ccc 1px, transparent 0);
		background-size: 40px 40px;
		background-position: 20px 20px;
	}
</style>
