<script lang="ts">
	import { SettingsService } from '$lib/gen'
	import WindmillIcon from './icons/WindmillIcon.svelte'

	export let subtitle: string | undefined = undefined
	export let title = 'Windmill'
	let version = ''

	SettingsService.backendVersion().then((x) => {
		version = x
	})
</script>

<div
	class="flex justify-center min-h-screen bg-gradient-to-r from-gray-900 to-blue-900 pt-10 relative"
>
	<div class="w-10/12 md:w-7/12 lg:w-6/12 xl:4/12 m-auto z-10">
		<div class="border rounded-md py-4 px-16 shadow-md bg-gray-50">
			<h1 class="justify-center text-center font-medium pb-4">
				{title}
			</h1>
			{#if subtitle}
				<p class="justify-center text-center font-medium pb-2 text-xs">
					{subtitle}
				</p>
			{/if}
			<slot />
		</div>

		<!-- empty row to make the form a little bit above vertical centering-->
		<div class="py-12" />
	</div>
	<div class="hidden lg:block absolute top-20 right-50">
		<div class="animate-[spin_100s_linear_infinite] ">
			<WindmillIcon white height="100px" width="100px" />
		</div>
		<h2 class="text-center pt-2 text-gray-100">Windmill</h2>
	</div>
	<div class="absolute top-0 right-0 text-2xs text-gray-300 italic px-3 py-1">
		<span class="font-mono">{version}</span>
	</div>
</div>
