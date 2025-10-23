<script lang="ts">
	import { setLicense } from '$lib/enterpriseUtils'
	import { enterpriseLicense, whitelabelNameStore } from '$lib/stores'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import LoginPageHeader from './LoginPageHeader.svelte'

	export let subtitle: string | undefined = undefined
	export let title = 'Windmill'
	export let disableLogo = false
	export let large = false

	setLicense()
</script>

<div class="center-center min-h-screen p-4 relative bg-surface-secondary">
	<div class="flex flex-col gap-2 items-center w-full">
		{#if (!disableLogo && !$enterpriseLicense) || !$whitelabelNameStore}
			<div class="hidden lg:block">
				<div>
					<WindmillIcon height="100px" width="100px" spin="slow" />
				</div>
				<h2 class="text-center py-2 text-primary">Windmill</h2>
			</div>
		{/if}

		<div
			class="rounded-md bg-surface w-full {large
				? 'max-w-5xl'
				: 'max-w-[640px]'} p-4 sm:py-8 sm:px-10 mb-6 md:mb-20 z-10"
		>
			<div class="mb-10">
				<h1 class="text-center text-lg text-emphasis font-semibold">
					{title}
				</h1>
				{#if subtitle}
					<p class="text-sm font-semibold text-emphasis text-center mt-2">
						{subtitle}
					</p>
				{/if}
			</div>
			<slot />
		</div>
	</div>

	<LoginPageHeader />
</div>
