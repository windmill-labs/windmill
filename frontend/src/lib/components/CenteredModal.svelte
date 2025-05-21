<script lang="ts">
	import { setLicense } from '$lib/enterpriseUtils'
	import { enterpriseLicense, whitelabelNameStore } from '$lib/stores'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import LoginPageHeader from './LoginPageHeader.svelte'

	let { subtitle, title, disableLogo, large } = $props()

	setLicense()
</script>

<div class="center-center min-h-screen p-4 relative bg-surface-secondary">
	<div class="flex flex-col gap-2 items-center w-full">
		{#if (!disableLogo && !$enterpriseLicense) || !$whitelabelNameStore}
			<div class="hidden lg:block">
				<div>
					<WindmillIcon height="100px" width="100px" spin="slow" />
				</div>
				<h2 class="text-center pt-2 text-primary">Windmill</h2>
			</div>
		{/if}

		<div
			class="border rounded-md shadow-md bg-surface w-full {large
				? 'max-w-5xl'
				: 'max-w-[640px]'} p-4 sm:py-8 sm:px-10 mb-6 md:mb-20 z-10"
		>
			<div class="mb-10">
				<h1 class="text-center text-primary">
					{title}
				</h1>
				{#if subtitle}
					<p class="text-sm text-center text-secondary mt-2">
						{subtitle}
					</p>
				{/if}
			</div>
			<slot />
		</div>
	</div>

	<LoginPageHeader />
</div>
