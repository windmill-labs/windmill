<script lang="ts">
	import PanelSection from './common/PanelSection.svelte'
	import type { RichConfigurations } from '../../types'
	import { Button } from '$lib/components/common'
	import { RefreshCcw } from 'lucide-svelte'

	export let configuration: RichConfigurations
</script>

<PanelSection title={'Helpers'}>
	<div>
		<Button size="xs" startIcon={{ icon: RefreshCcw }} color="dark">
			Sync columns definitions with table
		</Button>
	</div>
</PanelSection>
