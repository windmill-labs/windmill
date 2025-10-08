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

<button
	{title}
	{id}
	type="button"
	class={'w-[17.5px] h-[17.5px] flex items-center justify-center text-primary border border-border-normal bg-surface-secondary hover:opacity-60 transition-opacity rounded-md'}
	onpointerdown={onPress}
>
	<Icon size={12} />
</button>
