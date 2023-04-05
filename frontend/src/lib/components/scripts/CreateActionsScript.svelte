<script lang="ts">
	import { faPlus } from '@fortawesome/free-solid-svg-icons'

	import { Code2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<Button size="sm" spacingSize="xl" startIcon={{ icon: faPlus }} href="/scripts/add"
		>Script <Code2 class="ml-1.5" size={18} /></Button
	>
	<!-- <ButtonPopup size="sm" spacingSize="xl" startIcon={{ icon: faPlus }} href="/scripts/add">
		<svelte:fragment slot="main">Script <Code2 class="ml-1.5" size={18} /></svelte:fragment>
		<ButtonPopupItem on:click={() => drawer?.toggleDrawer?.()}>
			Import from template
		</ButtonPopupItem>
	</ButtonPopup> -->
</div>
