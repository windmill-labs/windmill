<script lang="ts">
	import { FlowService, type Flow } from '../../../gen';

	import { page } from '$app/stores';
	import { workspaceStore } from '../../../stores';
	import FlowBuilder from '../../components/FlowBuilder.svelte';
	import { emptySchema } from '../../../utils';

	const initialState = $page.url.searchParams.get('state');
	let flowLoadedFromUrl = initialState != undefined ? JSON.parse(atob(initialState)) : undefined;

	let flow: Flow = {
		path: $page.params.path,
		summary: '',
		edited_by: '',
		edited_at: '',
		value: { modules: [] },
		archived: false,
		extra_perms: {},
		schema: emptySchema()
	};

	let initialPath: string = '';

	async function loadFlow(): Promise<void> {
		flow =
			flowLoadedFromUrl != undefined && flowLoadedFromUrl.path == flow.path
				? flowLoadedFromUrl
				: await FlowService.getFlowByPath({
						workspace: $workspaceStore!,
						path: flow.path
				  });
		initialPath = flow.path;
	}

	$: {
		if ($workspaceStore) {
			loadFlow();
		}
	}
</script>

<FlowBuilder {initialPath} {flow} />
