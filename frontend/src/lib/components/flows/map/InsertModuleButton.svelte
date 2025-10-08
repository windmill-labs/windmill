<script lang="ts">
	import type { IconType } from '$lib/utils'
	import { Cross } from 'lucide-svelte'

	type Props = {
		onPress: () => void
		Icon?: IconType
		title?: string
		id?: string
	}

	let { onPress, Icon = Cross, title, id }: Props = $props()
</script>

<button
	{title}
	{id}
	type="button"
	class={'w-[17.5px] h-[17.5px] flex items-center justify-center !outline-[1px] outline dark:outline-gray-500 outline-gray-300 text-secondary bg-surface focus:outline-none hover:bg-surface-hover rounded'}
	onpointerdown={onPress}
>
	<Icon size={12} />
</button>
