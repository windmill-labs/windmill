<!-- Used to avoid height jitter when loading monaco asynchronously -->

<script lang="ts">
	type Props = {
		code: string
		fontSize: number
		marginLeft?: string
		className?: string
	}

	let { code, fontSize, marginLeft = '23px', className = '' }: Props = $props()
</script>

<div
	class={'w-full pr-14 z-9 mt-[-1px] mb-[1px] leading-5 overflow-x-auto mycontainer ' + className}
	style="padding-left: {marginLeft};"
>
	<code
		class="text-nowrap whitespace-pre-wrap text-ellipsis overflow-clip text-tertiary"
		style="font-size: {fontSize}px;"
	>
		{code}
	</code>
</div>

<style>
	.mycontainer::-webkit-scrollbar {
		/* Try to match monaco's scrollbar */
		height: 12px;
	}
	.mycontainer::-webkit-scrollbar-thumb {
		background-color: transparent;
	}
</style>
