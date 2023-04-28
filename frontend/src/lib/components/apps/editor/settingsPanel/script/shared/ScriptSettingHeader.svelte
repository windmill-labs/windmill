<script lang="ts">
	import { FunctionSquare } from 'lucide-svelte'
	import type { ActionType } from '../ComponentScriptSettings.svelte'
	import ScriptSettingsActions from './ScriptSettingsActions.svelte'

	export let name: string
	export let actions: ActionType[] = []
</script>

<div class="flex flex-row p-2 border-gray-200 justify-between bg-blue-50/60">
	<div class="flex flex-row gap-2 items-center">
		<FunctionSquare size={16} color="blue" />
		<span class="text-xs font-semibold truncate">
			{name}
		</span>
	</div>

	<ScriptSettingsActions {actions} />
</div>
