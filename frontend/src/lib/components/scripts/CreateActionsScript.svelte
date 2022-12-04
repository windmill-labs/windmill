<script lang="ts">
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { ButtonPopup, ButtonPopupItem } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import WorkspaceScriptPicker from '../flows/pickers/WorkspaceScriptPicker.svelte'
	import { goto } from '$app/navigation'

	let drawer: Drawer | undefined = undefined
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<ButtonPopup size="sm" spacingSize="xl" startIcon={{ icon: faPlus }} href="/scripts/add">
		<svelte:fragment slot="main">New Script</svelte:fragment>
		<ButtonPopupItem on:click={() => drawer?.toggleDrawer?.()}>
			Import from template
		</ButtonPopupItem>
	</ButtonPopup>
</div>

<!-- Template script list -->
<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Pick a template" on:close={() => drawer?.toggleDrawer?.()}>
		<WorkspaceScriptPicker
			isTemplate
			on:pick={(e) => goto(`/scripts/add?template=${e.detail.path}`)}
		/>
	</DrawerContent>
</Drawer>
