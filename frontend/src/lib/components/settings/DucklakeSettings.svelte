<script lang="ts">
	import { ConfigService } from '$lib/gen'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { Database } from 'lucide-svelte'
	import { Alert } from '../common'
	import Button from '../common/button/Button.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import { sendUserToast } from '$lib/toast'

	let dbResource: string | undefined = $state(undefined)

	const catalogExists = usePromise(ConfigService.ducklakeCatalogExists)
	const needsDbInit = $derived(!dbResource && catalogExists.status === 'ok' && !catalogExists.value)
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

<ResourcePicker
	resourceType="postgresql"
	placeholder="Windmill database (default)"
	bind:value={dbResource}
/>

{#if needsDbInit}
	<Alert
		class="mt-3"
		type="warning"
		title="Using the default windmill database requires ducklake_catalog initialization"
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
