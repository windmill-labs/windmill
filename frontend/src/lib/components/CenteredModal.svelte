<script lang="ts">
	import { SettingsService } from '$lib/gen'
	import { onMount } from 'svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'

	export let subtitle: string | undefined = undefined
	export let title = 'Windmill'
	export let disableLogo = false
	let version = ''

	onMount(async () => {
		version = await SettingsService.backendVersion()
	})
</script>

<div class="center-center min-h-screen p-4 relative bg-white">
	<div
		class="border rounded-md shadow-md bg-white w-full max-w-[640px]
	p-4 sm:py-8 sm:px-10 mb-6 md:mb-20 z-10"
	>
		<div class="mb-10">
			<h1 class="text-center">
				{title}
			</h1>
			{#if subtitle}
				<p class="text-sm text-center text-gray-600 mt-2">
					{subtitle}
				</p>
			{/if}
		</div>
		<slot />
	</div>
	{#if !disableLogo}
		<div class="hidden lg:block absolute top-10 right-50">
			<div class="animate-[spin_50s_linear_infinite]">
				<WindmillIcon height="100px" width="100px" />
			</div>
			<h2 class="text-center pt-2 text-gray-800">Windmill</h2>
		</div>
	{/if}
	<div class="absolute top-0 right-0 text-2xs text-gray-800 italic px-3 py-1">
		<span class="font-mono">{version}</span>
	</div>
</div>
