<script lang="ts">
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane } from 'svelte-splitpanes'
</script>

<SplitPanesWrapper>
	<Pane />
	<Pane>
		<div class="p-4">
			<div class="grid grid-cols-2 gap-4">
				<div class="border flex p-2 h-16"> Form </div>
				<div class="border flex p-2 h-16">01</div>
				<div class="border flex p-2 h-16">01</div>
				<div class="border flex p-2 h-16">01</div>
				<div class="border flex p-2 h-16">01</div>
			</div>
		</div>
	</Pane>
</SplitPanesWrapper>
