<script lang="ts">
	import { NODE } from '../../util'

	import { createEventDispatcher } from 'svelte'

	import TriggersBadge from './TriggersBadge.svelte'

	export let path: string
	export let newItem: boolean
	export let isFlow: boolean
	export let selected: boolean

	const dispatch = createEventDispatcher()
</script>

<div style={`width: ${NODE.width}px;`}>
	<div class="flex flex-row mx-auto w-min">
		<button
			class="flex flex-row gap-2 px-2 border p-1 rounded-md bg-surface shadow-md items-center {selected
				? 'outline outline-offset-1 outline-1  outline-slate-900 dark:bg-white/5 dark:outline-slate-800/60 dark:border-gray-400'
				: ''}"
			on:click={() => {
				dispatch('select')
			}}
		>
			<div class="flex flex-col">
				<div class="flex flex-row items-center text-2xs font-normal"> Triggers </div>
			</div>
			<TriggersBadge showOnlyWithCount={false} {path} {newItem} {isFlow} {selected} on:select />
		</button>
	</div>
</div>
