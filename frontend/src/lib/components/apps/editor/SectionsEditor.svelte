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
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import ComponentEditor from './ComponentEditor.svelte'
	import { classNames } from '$lib/utils'
	import RunFormComponent from '../components/RunFormComponent.svelte'
	import DisplayComponent from '../components/DisplayComponent.svelte'

	export let sections: AppSection[]

	export let mode: EditorMode = 'width'

	const flipDurationMs = 200
	const { selection, staticOutputs, app } = getContext<AppEditorContext>('AppEditorContext')

	function handleSort(e) {
		sections = e.detail.items
	}

	function addEmptySection() {
		sections = [
			...sections,
			{
				components: [],
				columns: 3,
				id: getNextId(sections.map((s) => s.id)),
				title: 'New section',
				description: 'section'
			}
		]
	}

	const numberToTailwindWidthMap = {
		1: 'w-[1%]',
		2: 'w-[2%]',
		3: 'w-[3%]',
		4: 'w-[4%]',
		5: 'w-[5%]',
		6: 'w-[6%]',
		7: 'w-[7%]',
		8: 'w-[8%]',
		9: 'w-[9%]',
		10: 'w-[10%]',
		11: 'w-[11%]',
		12: 'w-[12%]',
		13: 'w-[13%]',
		14: 'w-[14%]',
		15: 'w-[15%]',
		16: 'w-[16%]',
		17: 'w-[17%]',
		18: 'w-[18%]',
		19: 'w-[19%]',
		20: 'w-[20%]',
		21: 'w-[21%]',
		22: 'w-[22%]',
		23: 'w-[23%]',
		24: 'w-[24%]',
		25: 'w-[25%]',
		26: 'w-[26%]',
		27: 'w-[27%]',
		28: 'w-[28%]',
		29: 'w-[29%]',
		30: 'w-[30%]',
		31: 'w-[31%]',
		32: 'w-[32%]',
		33: 'w-[33%]',
		34: 'w-[34%]',
		35: 'w-[35%]',
		36: 'w-[36%]',
		37: 'w-[37%]',
		38: 'w-[38%]',
		39: 'w-[39%]',
		40: 'w-[40%]',
		41: 'w-[41%]',
		42: 'w-[42%]',
		43: 'w-[43%]',
		44: 'w-[44%]',
		45: 'w-[45%]',
		46: 'w-[46%]',
		47: 'w-[47%]',
		48: 'w-[48%]',
		49: 'w-[49%]',
		50: 'w-[50%]',
		51: 'w-[51%]',
		52: 'w-[52%]',
		53: 'w-[53%]',
		54: 'w-[54%]',
		55: 'w-[55%]',
		56: 'w-[56%]',
		57: 'w-[57%]',
		58: 'w-[58%]',
		59: 'w-[59%]',
		60: 'w-[60%]',
		61: 'w-[61%]',
		62: 'w-[62%]',
		63: 'w-[63%]',
		64: 'w-[64%]',
		65: 'w-[65%]',
		66: 'w-[66%]',
		67: 'w-[67%]',
		68: 'w-[68%]',
		69: 'w-[69%]',
		70: 'w-[70%]',
		71: 'w-[71%]',
		72: 'w-[72%]',
		73: 'w-[73%]',
		74: 'w-[74%]',
		75: 'w-[75%]',
		76: 'w-[76%]',
		77: 'w-[77%]',
		78: 'w-[78%]',
		79: 'w-[79%]',
		80: 'w-[80%]',
		81: 'w-[81%]',
		82: 'w-[82%]',
		83: 'w-[83%]',
		84: 'w-[84%]',
		85: 'w-[85%]',
		86: 'w-[86%]',
		87: 'w-[87%]',
		88: 'w-[88%]',
		89: 'w-[89%]',
		90: 'w-[90%]',
		91: 'w-[91%]',
		92: 'w-[92%]',
		93: 'w-[93%]',
		94: 'w-[94%]',
		95: 'w-[95%]',
		96: 'w-[96%]',
		97: 'w-[97%]',
		98: 'w-[98%]',
		99: 'w-[99%]',
		100: 'w-[100%]'
	}
</script>

<div class="dotted-background h-full w-full p-8">
	<div
		class="flex-col flex justify-start gap-8 pt-8 mb-8"
		use:dndzone={{
			items: sections,
			flipDurationMs,
			type: 'section',
			dragDisabled: mode === 'width',
			dropTargetStyle: {
				outline: 'dashed blue',
				outlineOffset: '8px'
			}
		}}
		on:consider={handleSort}
		on:finalize={handleSort}
	>
		{#each sections as section, sectionIndex (section.id)}
			{@const selected = $selection?.sectionIndex === sectionIndex}
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<section
				animate:flip={{ duration: flipDurationMs }}
				on:click={() => {
					$selection = { sectionIndex, componentIndex: undefined }
				}}
			>
				{#if mode !== 'preview'}
					<span
						class={classNames(
							'text-white px-2 text-sm py-1 font-bold',
							selected ? 'bg-indigo-500' : 'bg-gray-500'
						)}
					>
						Section {sectionIndex + 1}
						<Badge>{section.id}</Badge>
					</span>
				{/if}
				{#if mode === 'dnd'}
					<ComponentsEditor
						bind:components={section.components}
						columns={section.columns}
						{sectionIndex}
					/>
				{:else if mode === 'preview'}
					<div class="w-full flex bg-white">
						{#each section.components as component}
							<div class={classNames(numberToTailwindWidthMap[Math.round(component.width)])}>
								{#if component.type === 'runformcomponent'}
									<RunFormComponent
										{...component}
										bind:staticOutputs={$staticOutputs[component.id]}
										triggerable={$app.policy?.triggerables?.[component.id]}
									/>
								{:else if component.type === 'displaycomponent'}
									<DisplayComponent
										{...component}
										bind:staticOutputs={$staticOutputs[component.id]}
									/>
								{/if}
							</div>
						{/each}
					</div>
				{:else if mode === 'width'}
					<div
						class="h-80 w-full rounded-b-sm flex-row gap-4 p-4 flex border-2 border-gray-200 bg-white cursor-pointer "
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
	{#if mode !== 'preview'}
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
	{/if}
</div>

<style>
	.dotted-background {
		background-image: radial-gradient(circle at 1px 1px, #ccc 1px, transparent 0);
		background-size: 40px 40px;
		background-position: 20px 20px;
	}
</style>
