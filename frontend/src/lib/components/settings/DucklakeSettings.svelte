<script lang="ts">
	import { ConfigService } from '$lib/gen'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { Database } from 'lucide-svelte'
	import { Alert } from '../common'
	import Button from '../common/button/Button.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import { sendUserToast } from '$lib/toast'
	import Description from '../Description.svelte'
	import Toggle from '../Toggle.svelte'

	let enableInstanceCatalog = $state(false)

	const catalogExists = usePromise(ConfigService.ducklakeCatalogExists)
	const needsInstanceDbInit = $derived.by(
		() => enableInstanceCatalog && catalogExists.value === false
	)
	const onInitDucklakeCatalogDb = async () => {
		try {
			await ConfigService.initDucklakeCatalogDb()
			catalogExists.refresh()
			sendUserToast('Initialized ducklake_catalog database successfully', false)
		} catch (error) {
			sendUserToast("Couldn't initialize ducklake_catalog database: " + JSON.stringify(error), true)
			console.error('Error initializing ducklake_catalog database:', error)
		}
	}
</script>

<div class="flex flex-col gap-4 my-8">
	<div class="flex flex-col gap-1">
		<div class="text-primary text-lg font-semibold"> Ducklake</div>
		<Description link="https://www.windmill.dev/docs/core_concepts/ducklake">
			Windmill integrates with Ducklake.
		</Description>
	</div>
</div>
<!-- 
<ResourcePicker
	resourceType="postgresql"
	placeholder="Instance database (default)"
	keepSelectWhenEmpty
	bind:value={dbResource}
/> -->

<Toggle
	options={{
		right: 'Enable instance ducklake catalog',
		rightTooltip:
			"This will use the windmill instance database as the default ducklake catalog when doing ATTACH 'ducklake'"
	}}
	bind:checked={enableInstanceCatalog}
/>

{#if needsInstanceDbInit}
	<Alert
		class="mt-3"
		type="warning"
		title="Using the windmill instance database requires ducklake_catalog initialization"
	>
		<Button
			color="dark"
			endIcon={{ icon: Database }}
			wrapperClasses="max-w-fit"
			on:click={onInitDucklakeCatalogDb}
		>
			Create ducklake_catalog database
		</Button>
	</Alert>
{/if}
