<script lang="ts">
	import { Drawer, DrawerContent, ListRow } from '$lib/components/common'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import Version from '$lib/components/Version.svelte'
	import UserInfoSettings from '$lib/components/settings/UserInfoSettings.svelte'
	import {
		getDarkModeVariant,
		setDarkModeVariant,
		type DarkModeVariant
	} from '$lib/darkModeVariant'
	import { ACCOUNT_SETTINGS_LINKS } from './accountSettingsLinks'

	let { open = $bindable() }: { open?: boolean } = $props()

	let darkVariant = $state<DarkModeVariant>(getDarkModeVariant())

	const otherLinks = ACCOUNT_SETTINGS_LINKS.filter((l) => l.panel !== 'profile')
</script>

<Drawer bind:open size="600px" on:open={() => (darkVariant = getDarkModeVariant())}>
	<DrawerContent title="Profile" on:close={() => (open = false)}>
		<div class="flex flex-col gap-6 pb-8">
			<UserInfoSettings />

			<div class="flex flex-col gap-1">
				<div class="text-xs text-emphasis font-semibold flex items-center">
					Theme <DarkModeToggle forcedDarkMode={false} />
				</div>
				<div class="flex items-center gap-2">
					<span class="text-xs text-secondary">Dark variant</span>
					<ToggleButtonGroup
						selected={darkVariant}
						class="w-fit"
						onSelected={(v) => {
							darkVariant = v
							setDarkModeVariant(v)
						}}
					>
						{#snippet children({ item })}
							<ToggleButton value="default" label="Default" size="sm" {item} />
							<ToggleButton value="github" label="GitHub" size="sm" {item} />
						{/snippet}
					</ToggleButtonGroup>
				</div>
			</div>

			<div class="flex flex-col gap-1">
				<h2 class="text-emphasis text-sm font-semibold mb-1">More account settings</h2>
				{#each otherLinks as link (link.panel)}
					<ListRow onClick={() => (window.location.hash = link.href)}>
						{#snippet icon()}<link.icon size={16} />{/snippet}
						{#snippet title()}{link.label}{/snippet}
						{#snippet subtitle()}{link.description}{/snippet}
					</ListRow>
				{/each}
			</div>

			<div class="flex items-center gap-1 text-xs text-secondary">Windmill <Version /></div>
		</div>
	</DrawerContent>
</Drawer>
