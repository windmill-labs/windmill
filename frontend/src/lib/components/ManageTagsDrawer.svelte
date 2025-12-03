<script lang="ts">
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import AssignableTagsInner from './AssignableTagsInner.svelte'
	import DefaultTagsInner from './DefaultTagsInner.svelte'
	import { ExternalLink } from 'lucide-svelte'
	import { Section } from './common'

	interface Props {
		defaultTagPerWorkspace?: boolean | undefined
		defaultTagWorkspaces?: string[]
		onRefresh?: () => void
	}

	let {
		defaultTagPerWorkspace = $bindable(undefined),
		defaultTagWorkspaces = $bindable([]),
		onRefresh
	}: Props = $props()

	let drawer: Drawer | undefined = $state(undefined)

	export function openDrawer() {
		drawer?.openDrawer?.()
	}

	export function closeDrawer() {
		drawer?.closeDrawer?.()
	}

	export function toggleDrawer() {
		drawer?.toggleDrawer?.()
	}
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Manage tags" on:close={() => drawer?.closeDrawer?.()}>
		<div class="flex flex-col h-full gap-6">
			<!-- Overall Description -->
			<div class="text-xs font-normal text-secondary">
				Tags determine which worker group will execute a given job. Workers process only those jobs
				whose tags match those defined in their <a
					href="https://www.windmill.dev/docs/core_concepts/worker_groups"
					target="_blank">worker group <ExternalLink size={12} class="inline-block" /></a
				>
				configuration.
			</div>

			<!-- Content Sections -->
			<div class="flex flex-col gap-8 flex-1">
				<!-- Custom Tags Section -->
				<Section label="Custom tags">
					<AssignableTagsInner
						variant="drawer"
						on:refresh={() => {
							if (onRefresh) {
								onRefresh()
							}
						}}
					/>
				</Section>

				<!-- Default Tags Section -->
				<DefaultTagsInner bind:defaultTagPerWorkspace bind:defaultTagWorkspaces />

				<!-- Extra padding -->
				<div class="pb-10"></div>
			</div>
		</div>
	</DrawerContent>
</Drawer>
