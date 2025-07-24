<script lang="ts">
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Alert, DrawerContent } from '../common'
	import Button from '../common/button/Button.svelte'
	import Drawer from '../common/drawer/Drawer.svelte'
	import Path from '../Path.svelte'
	import { random_adj } from '../random_positive_adjetive'

	export function openDrawer() {
		let random_adjective = random_adj()
		resource_path = `u/${$userStore?.username ?? 'username'}/wm_${random_adjective}_ducklake_catalog`
		database_name = `wm_${random_adjective}_ducklake_catalog`
		drawer?.openDrawer()
	}

	let resource_path: string = $state('')
	let database_name: string = $state('')

	async function onConfirm() {
		try {
		} catch (e) {
			sendUserToast(e, true)
		}
	}

	let drawer: Drawer | undefined = $state()
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Create a new ducklake catalog">
		<div class="h-full flex flex-col gap-2">
			<Alert title="This will create a new database in the Windmill PostgreSQL instance :">
				<code class="flex items-center">
					<span class="shrink-0"> CREATE DATABASE </span>
					<input
						type="text"
						bind:value={database_name}
						placeholder="Database name"
						class="inline max-w-80 ml-3 mr-1"
					/>
					;
				</code>
			</Alert>

			<p class="mt-8 mb-3 text-sm"> The catalog will be accessible as this resource : </p>

			<Path initialPath={resource_path} kind="resource" bind:path={resource_path} />

			<div class="flex-1 flex items-end">
				<Button wrapperClasses="w-full" on:click={onConfirm}>Confirm</Button>
			</div>
		</div>
	</DrawerContent>
</Drawer>
