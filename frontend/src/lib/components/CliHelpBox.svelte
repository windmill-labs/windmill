<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import ClipboardPanel from './details/ClipboardPanel.svelte'
	import Section from './Section.svelte'

	let url = $derived(`${window.location.protocol}//${window.location.hostname}/`)
</script>

<Section label="CLI quick setup">
	<div class="text-xs" role="alert" id="dynamic-input-help-box">
		<ul class="pl-0 list-decimal list-inside flex flex-col gap-4">
			<li>
				<span class="font-medium">Install the latest wmill CLI from npm: </span>
				<div class="mt-1">
					<ClipboardPanel content={'npm install -g windmill-cli'} />
				</div>
			</li>

			<li>
				<span class="font-medium">Setup the wmill cli for this workspace & remote:</span>
				<div class="mt-1">
					<ClipboardPanel
						content={`wmill workspace add ${$workspaceStore} ${$workspaceStore} ${url}`}
					/>
				</div>
			</li>
			<li><span class="font-medium">Follow the prompts in your terminal</span></li>
			<li><span class="font-medium">Use the run command above!</span></li>
		</ul>
	</div>
</Section>
