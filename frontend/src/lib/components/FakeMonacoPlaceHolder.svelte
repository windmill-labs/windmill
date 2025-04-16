<!-- Used to avoid height jitter when loading monaco asynchronously -->

<script lang="ts">
	type Props = {
		code?: string
		fontSize: number
		marginLeft?: string
		className?: string
	}

	let { code, fontSize, marginLeft = '0', className = '' }: Props = $props()
</script>

<div
	class={'relative w-full pr-14 py-0 z-9 mt-[-1px] mb-[1px] dark:bg-[#272D38] overflow-y-clip overflow-x-auto mycontainer ' +
		className}
	style="padding-left: {marginLeft};"
>
	<div
		class="absolute inset-x-0 h-[22px] dark:bg-[#3A4252] mix-blend-lighten"
		style="margin-left: {marginLeft};"
	></div>
	<code
		class="text-nowrap whitespace-pre overflow-clip text-tertiary/50"
		style="font-size: {fontSize}px;"
	>
		{code}
		{#if code?.endsWith('\n')}
			&nbsp;
		{/if}
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
