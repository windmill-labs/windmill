<script lang="ts">
	import { Drawer, DrawerContent, Button } from '$lib/components/common'
	import SuperadminSettingsInner from './SuperadminSettingsInner.svelte'
	import Version from './Version.svelte'
	import Uptodate from './Uptodate.svelte'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { ExternalLink } from 'lucide-svelte'

	interface Props {
		disableChatOffset?: boolean
	}

	let { disableChatOffset = false }: Props = $props()

	let drawer: Drawer | undefined = $state()

	export function openDrawer() {
		drawer?.openDrawer()
	}

	export function closeDrawer() {
		drawer?.closeDrawer()
	}
</script>

<Drawer bind:this={drawer} size="1100px" {disableChatOffset}>
	<DrawerContent noPadding title="Instance settings" on:close={closeDrawer}>
		{#snippet actions()}
			<div class="text-xs text-primary flex items-center gap-1">
				Windmill <Version />
			</div>
			<Uptodate />
			{#if $workspaceStore !== 'admins'}
				<Button
					variant="default"
					size="xs"
					target="_blank"
					href="{base}/?workspace=admins"
					endIcon={{ icon: ExternalLink }}
				>
					Admins workspace
				</Button>
			{/if}
		{/snippet}
		<SuperadminSettingsInner {closeDrawer} showHeaderInfo={false} />
	</DrawerContent>
</Drawer>
