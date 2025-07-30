<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { CheckCircle2, ExternalLink, ArrowRight } from 'lucide-svelte'

	interface Props {
		open: boolean
	}

	let {
		open = $bindable(false)
	}: Props = $props()

</script>

<Modal bind:open title="Git Sync Connection Saved" class="sm:max-w-4xl">
	<div class="flex flex-col gap-6 p-6">
		<!-- Success header -->
		<div class="flex items-center gap-3">
			<div class="flex-shrink-0">
				<CheckCircle2 class="h-8 w-8 text-green-600" />
			</div>
			<div>
				<h3 class="text-lg font-semibold text-primary">Git sync connection saved successfully!</h3>
				<p class="text-sm text-secondary mt-1">Your repository is now configured to receive changes from Windmill.</p>
			</div>
		</div>


		<!-- Optional setup section -->
		<div class="bg-amber-50 border border-amber-200 rounded-lg p-4">
			<h4 class="font-medium text-amber-900 mb-2 flex items-center gap-2">
				<ArrowRight class="h-4 w-4" />
				Optional: Enable automatic deployment from Git to Windmill
			</h4>
			<p class="text-sm text-amber-800 mb-3">
				To automatically deploy changes from your Git repository back to Windmill (when PRs are merged), you can set up GitHub Actions or similar CI/CD workflows.
			</p>
			<div class="flex flex-col gap-2">
				<p class="text-sm text-amber-700">This setup enables:</p>
				<ul class="text-sm text-amber-700 ml-4 list-disc space-y-1">
					<li>Automatic deployment to Windmill when PRs are merged</li>
					<li>Full bidirectional sync between Git and Windmill</li>
				</ul>
				<div class="mt-3">
					<a
						href="https://www.windmill.dev/docs/advanced/deploy_gh_gl#github-actions-setup"
						target="_blank"
						class="text-sm text-amber-700 hover:text-amber-900 underline flex items-center gap-1"
					>
						<ExternalLink class="h-3 w-3" />
						Learn more
					</a>
				</div>
			</div>
		</div>

	</div>
</Modal>
