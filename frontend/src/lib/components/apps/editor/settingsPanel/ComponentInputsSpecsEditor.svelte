<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { ComponentInputsSpec } from '../../types'
	import DynamicInputEditor from './DynamicInputEditor.svelte'

	export let componentInputs: ComponentInputsSpec

	let openedProp = Object.keys(componentInputs)[0]
</script>

<div class="w-full flex flex-col gap-4">
	{#each Object.keys(componentInputs) as inputSpecKey}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div
			class={classNames(
				'w-full text-xs font-bold  border rounded-md py-1 px-2 cursor-pointer hover:bg-gray-800 hover:text-white transition-all',
				openedProp !== inputSpecKey ? 'bg-gray-200 ' : 'bg-gray-600 text-gray-300'
			)}
			on:click={() => {
				openedProp = inputSpecKey
			}}
		>
			{inputSpecKey}
		</div>
		{#if inputSpecKey === openedProp}
			<div class="flex flex-col w-full gap-2">
				<DynamicInputEditor bind:input={componentInputs[inputSpecKey]} />
			</div>
		{/if}
	{/each}
</div>
