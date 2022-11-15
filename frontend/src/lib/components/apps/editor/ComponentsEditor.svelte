<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { AppComponent, AppEditorContext } from '../types'
	import ComponentEditor from './ComponentEditor.svelte'
	import { dndzone } from 'svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import { classNames } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'

	export let components: AppComponent[]
	export let sectionIndex: number
	export let columns: number

	const flipDurationMs = 200
	const { selection, connectingInput, resizing, app } =
		getContext<AppEditorContext>('AppEditorContext')

	function handleDndConsider(event: CustomEvent<DndEvent<AppComponent>>) {
		components = event.detail.items.map((c) => {
			return {
				...c,
				width: Math.round(100 / event.detail.items.length)
			}
		})
	}

	function handleDndFinalize(event: CustomEvent<DndEvent<AppComponent>>) {
		components = event.detail.items.map((c) => {
			return {
				...c,
				width: Math.round(100 / event.detail.items.length)
			}
		})
	}

	// HACK
	onMount(() => {
		// get all div elements with the id "component"
		const divs = document.querySelectorAll<HTMLDivElement>('#component')
		// loop through the divs
		divs.forEach((div, index) => {
			// set component width
			div.style.width = `${Math.round(components[index].width)}%`
		})
	})

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

<div class="rounded-b-sm flex-row  flex bg-white cursor-pointer ">
	<div class="flex flex-row w-full">
		<div
			class={classNames('flex w-full gap-2 hover:bg-blue-100 p-4 rounded-md')}
			use:dndzone={{
				items: components,
				flipDurationMs,
				type: 'component',
				dropTargetStyle: {
					outline: '1px dashed blue',
					outlineOffset: '2px'
				},
				dragDisabled: components.length === 0 || $resizing,
				dropFromOthersDisabled: components.length === columns
			}}
			on:consider={handleDndConsider}
			on:finalize={handleDndFinalize}
		>
			{#if components.length > 0}
				{#each components as component, componentIndex (component.id)}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						class={classNames(numberToTailwindWidthMap[component.width], 'bg-green-100')}
						animate:flip={{ duration: flipDurationMs }}
						on:click|stopPropagation={() => {
							if (!$connectingInput.opened) {
								$selection = { componentIndex, sectionIndex }
							}
						}}
					>
						<ComponentEditor
							bind:component
							selected={componentIndex === $selection?.componentIndex &&
								sectionIndex === $selection?.sectionIndex}
						/>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
