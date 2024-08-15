<script lang="ts">
	import { Calendar, Plus, Split, Webhook } from 'lucide-svelte'
	import { getStateColor } from '../../util'
	import TriggerButton from './TriggerButton.svelte'

	export let darkMode: boolean = false
</script>

<div
	class="text-xs border-x border-t py-1 px-2 rounded-t-sm w-min border-surface-selected"
	style={`background-color: ${getStateColor(undefined, darkMode)};`}
>
	Triggers
</div>

<div
	class="flex flex-row gap-4 border p-2 rounded-b-lg rounded-r-lg border-surface-selected relative"
>
	<TriggerButton>
		<Webhook size={14} />
	</TriggerButton>
	<TriggerButton>
		<Calendar size={14} />
	</TriggerButton>
	<TriggerButton>
		<Split size={14} />
	</TriggerButton>

	<div class="absolute -right-10 top-1/2 -translate-y-1/2">
		<div
			class="bg-surface hover:bg-surface-hover rounded-full border center-center text-xs relative w-6 h-6 flex flex-row gap-2"
		>
			<Plus size={14} />
		</div>
	</div>
</div>
