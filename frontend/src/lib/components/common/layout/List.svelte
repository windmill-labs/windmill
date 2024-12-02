<script lang="ts">
	export let horizontal: boolean = false
	export let gap: 'none' | 'sm' | 'md' | 'lg' = 'sm'
	export let justify: 'start' | 'center' | 'end' | 'between' = 'start'
	export let full: boolean = false

	const gapMap = {
		none: '',
		sm: 'gap-2',
		md: 'gap-4',
		lg: 'gap-8'
	}

	const justifyMap = {
		start: 'justify-start',
		center: 'justify-center',
		end: 'justify-end',
		between: 'justify-between'
	}
</script>

{#if horizontal}
	<div
		class="flex flex-row w-full {gapMap[gap]} items-center {justifyMap[justify]} {full
			? 'h-full'
			: ''}"
	>
		<slot />
	</div>
{:else}
	<div
		class="flex flex-col h-full {gapMap[gap]} items-center {justifyMap[justify]} {full
			? 'w-full'
			: ''}"
	>
		<slot />
	</div>
{/if}
