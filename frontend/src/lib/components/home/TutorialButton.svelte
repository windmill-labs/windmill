<script lang="ts">
	import type { LucideIcon } from 'lucide-svelte'

	interface Props {
		icon: LucideIcon
		title: string
		description: string
		onclick: () => void
	}

	let { icon: Icon, title, description, onclick }: Props = $props()
</script>

<button
	onclick={onclick}
	class="block border rounded-lg px-6 py-5 bg-surface-secondary hover:bg-surface-hover transition-colors text-left"
>
	<div class="flex items-center gap-2 mb-2 text-primary">
		<Icon size={24} />
		<h2 class="text-base font-semibold">{title}</h2>
	</div>
	<p class="text-sm font-normal text-secondary">
		{description}
	</p>
</button>

