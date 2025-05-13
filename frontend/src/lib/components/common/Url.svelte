<script lang="ts">
	import { Badge } from '$lib/components/common'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'

	interface Props {
		url?: string
		disabled?: boolean
		label?: string
	}

	let { url = '', disabled = false, label = '' }: Props = $props()
</script>

<div class="flex justify-start w-full">
	<Badge color="gray" class="rounded-r-none h-[27px]">{label}</Badge>

	<ClipboardPanel
		content={url}
		class="rounded-l-none bg-surface border-none outline outline-2 outline-surface-secondary outline-offset-[-2px]"
		{disabled}
	/>
</div>
