<script lang="ts">
	import { FunctionSquare } from 'lucide-svelte'
	import type { ActionType } from '../ComponentScriptSettings.svelte'
	import ScriptSettingsActions from './ScriptSettingsActions.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		name: string
		actions?: ActionType[]
		noBorder?: boolean
	}

	let { name, actions = [], noBorder = false }: Props = $props()
</script>

<div
	class={twMerge(
		'flex flex-row px-2 py-1 justify-between bg-surface-secondary  ',
		noBorder ? '' : 'border rounded'
	)}
>
	<div class="flex flex-row gap-2 items-center min-w-0">
		<FunctionSquare size={16} color="#3b82f6" />
		<span class="text-xs font-semibold truncate">
			{name}
		</span>
	</div>

	<ScriptSettingsActions {actions} />
</div>
