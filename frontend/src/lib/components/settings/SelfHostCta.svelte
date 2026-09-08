<script lang="ts">
	import { Button } from '../common'
	import { Cloud, ExternalLink } from 'lucide-svelte'
	import DockerIcon from '../icons/DockerIcon.svelte'
	import KubernetesIcon from '../icons/KubernetesIcon.svelte'

	const SELF_HOST_DOCS = 'https://www.windmill.dev/docs/advanced/self_host'
</script>

<div class="box p-4 text-xs prose-sm rounded-md">
	<h2 class="mb-2">Prefer to self-host?</h2>
	<p class="text-sm mt-0 mb-4">
		Windmill is open source and runs on your own infrastructure, with no execution limits on the
		community edition.
		<a
			class="inline-flex items-center gap-1"
			href="{SELF_HOST_DOCS}#architecture"
			target="_blank"
			rel="noopener"
		>
			Read the self-hosting guide<ExternalLink size={12} />
		</a>
	</p>
	<div class="not-prose flex flex-wrap items-center gap-2">
		<Button variant="default" unifiedSize="sm" href="{SELF_HOST_DOCS}#docker" target="_blank">
			<DockerIcon width="16px" height="16px" />
			Docker Compose
		</Button>
		<Button variant="default" unifiedSize="sm" href="{SELF_HOST_DOCS}#helm-chart" target="_blank">
			<KubernetesIcon width="16px" height="16px" />
			Kubernetes (Helm)
		</Button>
		<Button
			variant="default"
			unifiedSize="sm"
			href="{SELF_HOST_DOCS}#cloud-provider-specific-guides"
			target="_blank"
			startIcon={{ icon: Cloud }}
		>
			AWS, GCP, Azure and more
		</Button>
	</div>
</div>
