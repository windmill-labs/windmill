<!-- Used to avoid height jitter when loading monaco asynchronously -->

<script lang="ts">
	type Props = {
		code: string
		fontSize: number
		marginLeft?: string
	}

	let { code, fontSize, marginLeft = '23px' }: Props = $props()
</script>

<div
	class="w-full pr-14 z-9 mt-[-1px] mb-[1px] leading-5 overflow-x-auto mycontainer"
	style="margin-left: {marginLeft};"
>
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
