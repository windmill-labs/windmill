<script lang="ts">
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import Uptodate from './Uptodate.svelte'
	import Version from './Version.svelte'

	export let subtitle: string | undefined = undefined
	export let title = 'Windmill'
	export let disableLogo = false
</script>

<div class="center-center min-h-screen p-4 relative bg-gray-50">
	<div class="flex flex-col gap-2 items-center w-full">
		{#if !disableLogo}
			<div class="hidden lg:block">
				<div>
					<WindmillIcon height="100px" width="100px" spin="slow" />
				</div>
				<h2 class="text-center pt-2 text-gray-800">Windmill</h2>
			</div>
		{/if}

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
	</div>

	<div class="absolute top-0 right-0 text-2xs text-gray-600 italic px-3 py-1">
		<span class="font-mono flex flex-col">
			<div>Windmill <Version /></div><div><Uptodate /></div>
		</span>
	</div>
</div>
