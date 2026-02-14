<script lang="ts">
	import { Drawer, DrawerContent, Button } from '$lib/components/common'
	import SuperadminSettingsInner from './SuperadminSettingsInner.svelte'
	import Version from './Version.svelte'
	import MeltTooltip from './meltComponents/Tooltip.svelte'
	import Toggle from './Toggle.svelte'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { ExternalLink } from 'lucide-svelte'
	import { SettingsService } from '$lib/gen'
	import { isCloudHosted } from '$lib/cloud'

	interface Props {
		disableChatOffset?: boolean
	}

	let { disableChatOffset = false }: Props = $props()

	let drawer: Drawer | undefined = $state()
	let uptodateVersion: string | undefined = $state(undefined)
	let yamlMode = $state(false)

	async function loadUptodate() {
		try {
			const res = await SettingsService.backendUptodate()
			if (res != 'yes') {
				const parts = res.split(' -> ')
				uptodateVersion = parts.length > 1 ? parts[parts.length - 1] : res
			}
		} catch {}
	}
	loadUptodate()

	export function openDrawer() {
		drawer?.openDrawer()
	}

	export function closeDrawer() {
		drawer?.closeDrawer()
	}
</script>

<Drawer bind:this={drawer} size="1200px" {disableChatOffset}>
	<DrawerContent noPadding overflow_y={false} title="Instance settings" on:close={closeDrawer}>
		{#snippet actions()}
			<MeltTooltip disablePopup={!uptodateVersion}>
				<div class="text-xs text-secondary flex items-center gap-1">
					Windmill <Version />
					{#if uptodateVersion}
						<span class="text-accent">→ {uptodateVersion}</span>
					{/if}
				</div>
				<svelte:fragment slot="text">
					{#if isCloudHosted()}
						The cloud version is updated daily.
					{:else}
						How to update?<br />
						- docker: <code>docker compose up -d</code><br />
						- <a href="https://github.com/windmill-labs/windmill-helm-charts#install">helm</a>
					{/if}
				</svelte:fragment>
			</MeltTooltip>
			{#if $workspaceStore !== 'admins'}
				<Button
					variant="default"
					size="xs"
					target="_blank"
					href="{base}/?workspace=admins"
					endIcon={{ icon: ExternalLink }}
					wrapperClasses="ml-2"
				>
					Admins workspace
				</Button>
			{/if}
			<Toggle
				bind:checked={yamlMode}
				options={{ right: 'YAML' }}
				size="sm"
			/>
		{/snippet}
		<SuperadminSettingsInner {closeDrawer} showHeaderInfo={false} bind:yamlMode />
	</DrawerContent>
</Drawer>
