<script lang="ts">
	import { Badge } from '$lib/components/common'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'

	export let url = ''
	export let disabled = false
	export let label = ''
</script>

<div class="flex justify-start w-full">
	<Badge color="gray" class="rounded-r-none h-[27px]">{label}</Badge>

	<ClipboardPanel
		content={url}
		class="rounded-l-none bg-surface border-none outline outline-2 outline-surface-secondary outline-offset-[-2px]"
		{disabled}
	/>
</div>
