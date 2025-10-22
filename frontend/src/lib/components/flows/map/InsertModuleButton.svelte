<script lang="ts">
	import type { IconType } from '$lib/utils'
	import { Plus } from 'lucide-svelte'

	type Props = {
		onPress: () => void
		Icon?: IconType
		title?: string
		id?: string
	}

	let { onPress, Icon = Plus, title, id }: Props = $props()
</script>

<button class="center-center p-2 group" onpointerdown={onPress}>
	<div
		{title}
		{id}
		class={'w-[20px] h-[20px] flex items-center justify-center text-primary border border-border-normal bg-surface-secondary group-hover:bg-surface-accent-primary group-hover:text-white group-hover:border-none transition-all rounded-md'}
	>
		<Icon size={12} />
	</div>
</button>
