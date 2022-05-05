<script lang="ts">
	import { SettingsService } from '../../gen';

	export let subtitle: string | undefined = undefined;
	export let title = 'Welcome to windmill.dev (alpha)';
	let version = '';

	SettingsService.backendVersion().then((x) => {
		version = x;
	});
</script>

<div class="flex justify-center h-screen bg-gray-50">
	<div class="w-10/12 md:w-7/12 lg:w-4/12 xl:3/12 m-auto">
		<h1 class="justify-center text-center font-medium pb-2 text-gray-700">
			{title}
		</h1>
		{#if subtitle}
			<p class="justify-center text-center font-medium pb-2 text-gray-700 text-xs">
				{subtitle}
			</p>
		{/if}
		<div class="border rounded-md py-12 px-16 shadow-md bg-white">
			<slot />
		</div>

		<!-- empty row to make the form a little bit above vertical centering-->
		<div class="py-12" />
	</div>
	<div class="absolute bottom-0 right-0 text-2xs text-gray-500 italic px-3 py-1">
		windmill.dev backend v. <span class="font-mono">{version}</span>
	</div>
</div>
