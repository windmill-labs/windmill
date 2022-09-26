<!-- <div
	class={'bg-gray-200/40 flex flex-wrap justify-between items-center gap-4 px-4 sm:px-6 md:px-8 py-3 ' +
		$$props.class}
> -->
<div
	class={'bg-white flex flex-wrap justify-between items-center gap-4 px-4 sm:px-6 md:px-8 py-3 ' +
		$$props.class}
>
	{#if $$slots.left}
		<div class="flex flex-wrap items-center gap-2">
			<slot name="left" />
		</div>
	{/if}
	{#if $$slots.middle}
		<div class="flex flex-wrap items-center gap-2">
			<slot name="middle" />
		</div>
	{/if}
	{#if $$slots.right}
		<div class="flex flex-wrap items-center gap-2">
			<slot name="right" />
		</div>
	{/if}
</div>
