<script lang="ts">
	import { setLicense } from '$lib/enterpriseUtils'
	import { enterpriseLicense, whitelabelNameStore } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import LoginPageHeader from './LoginPageHeader.svelte'

	interface Props {
		subtitle?: string | undefined
		title?: string
		disableLogo?: boolean
		large?: boolean
		centerVertically?: boolean
		loading?: boolean
		containOverflow?: boolean
		children?: import('svelte').Snippet
	}

	let {
		subtitle = undefined,
		title = 'Windmill',
		disableLogo = false,
		large = false,
		centerVertically = true,
		loading = false,
		containOverflow = false,
		children
	}: Props = $props()

	setLicense()

	let height = $state(0)
</script>

<div
	class="flex justify-center h-screen p-4 relative bg-surface-secondary {containOverflow
		? 'overflow-hidden'
		: 'overflow-auto'}"
	class:items-center={centerVertically}
	style="scrollbar-gutter: stable both-edges;"
	bind:clientHeight={height}
>
	<div
		class={twMerge(
			'flex flex-col gap-2 items-center w-full pb-8',
			containOverflow ? 'min-h-0' : 'h-fit',
			containOverflow ? '' : height > 1080 ? 'pt-28' : 'pt-12'
		)}
	>
		{#if (!disableLogo && !$enterpriseLicense) || !$whitelabelNameStore}
			<div class="hidden lg:block">
				<div>
					<WindmillIcon size={centerVertically ? 64 : 48} spin={loading ? 'fast' : 'slow'} />
				</div>
			</div>
		{:else}
			<div class="pt-8"></div>
		{/if}

		<div class="mb-4">
			<h1 class="text-center text-lg text-emphasis font-semibold">
				{title}
			</h1>
			{#if subtitle}
				<p class="text-xs font-normal text-primary text-center mt-2">
					{subtitle}
				</p>
			{/if}
		</div>

		{#if children}
			<div
				class="rounded-md bg-surface w-full {large
					? 'max-w-5xl'
					: 'max-w-[640px]'} p-4 sm:py-8 sm:px-10 z-10 {containOverflow
					? 'flex-1 min-h-0 flex flex-col'
					: ''}"
			>
				{@render children()}
			</div>
		{/if}
	</div>

	<LoginPageHeader />
</div>
