import { isCloudHosted } from '$lib/cloud'

import { superadmin } from '$lib/stores'
import { derived } from 'svelte/store'

export let isCustomInstanceDbEnabled = derived(
	[superadmin],
	([superadmin_]) => superadmin_ && !isCloudHosted()
)
