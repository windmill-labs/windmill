<script lang="ts">
	import PanelSection from './common/PanelSection.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCcw } from 'lucide-svelte'
</script>

<PanelSection title={'Helpers'}>
	<div>
		<Button size="xs" startIcon={{ icon: RefreshCcw }} color="dark">
			Sync columns definitions with table
		</Button>
	</div>
</PanelSection>
