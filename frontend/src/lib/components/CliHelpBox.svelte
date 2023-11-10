<script lang="ts">
	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import ClipboardPanel from './details/ClipboardPanel.svelte'

	$: url = `${$page.url.protocol}//${$page.url.hostname}/`
</script>

<div>
	<div class="text-sm font-semibold">CLI quick setup </div>
	<div class="text-sm" role="alert" id="dynamic-input-help-box">
		<ul class="pl-0 pt-2 list-decimal list-inside flex flex-col gap-2">
			<li>
				Install the latest wmill CLI from deno.land:
				<ClipboardPanel content={'deno install --unstable -A https://deno.land/x/wmill/main.ts'} />
			</li>

			<li>
				Setup the wmill cli for this workspace & remote:
				<ClipboardPanel
					content={`wmill workspace add ${$workspaceStore} ${$workspaceStore} ${url}`}
				/>
			</li>
			<li>Follow the prompts in your terminal</li>
			<li>Use the run command above!</li>
		</ul>
	</div>
</div>
