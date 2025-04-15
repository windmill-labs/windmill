<!-- Used to avoid height jitter when loading monaco asynchronously -->

<script lang="ts">
	type Props = {
		code: string
		fontSize: number
	}

	let { code, fontSize }: Props = $props()
</script>

<div class="w-full pl-6 pr-14 z-9 leading-5 overflow-x-auto mycontainer">
	<code
		class="text-nowrap whitespace-pre-wrap text-ellipsis overflow-clip"
		style="font-size: {fontSize}px;">{code}</code
	>
</div>

<style>
	.mycontainer::-webkit-scrollbar {
		/* Try to match monaco's scrollbar */
		height: 12px;
	}
</style>
