<!-- Used to avoid height jitter when loading monaco asynchronously -->

<script lang="ts">
	type Props = {
		code: string
		fontSize: number
		marginLeft?: string
		className?: string
	}

	let { code, fontSize, marginLeft = '1.44em', className = '' }: Props = $props()
</script>

<div
	class={'relative w-full pr-14 z-9 mt-[-0.026em] mb-[0.026em] leading-5 overflow-y-clip overflow-x-auto mycontainer ' +
		className}
	style="padding-left: {marginLeft};"
>
	<div
		class="absolute inset-x-0 h-[1.36em] dark:bg-[#3A4252] mix-blend-lighten"
		style="margin-left: {marginLeft};"
	></div>
	<code
		class="text-nowrap whitespace-pre-wrap text-ellipsis overflow-clip text-tertiary/50"
		style="font-size: {fontSize / 16}em;"
	>
		{code}
	</code>
</div>

<style>
	.mycontainer::-webkit-scrollbar {
		/* Try to match monaco's scrollbar */
		height: 0.75px;
	}
	.mycontainer::-webkit-scrollbar-thumb {
		background-color: transparent;
	}
</style>
