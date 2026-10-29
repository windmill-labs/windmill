import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'

/** What "Customize sidebar" in the settings menu changes, per browser. Module-level so the
 * menu that flips a choice and the sidebar that reads it share one value. */
export const sidebarPrefs = {
	/** The starred items as a section of their own. */
	favorites: useLocalStorageValue('windmill_sidebar_show_favorites', false, 'boolean'),
	/** Variables, Resources and Assets as a collapsible Data section instead of one Data menu. */
	splitData: useLocalStorageValue('windmill_sidebar_split_data', true, 'boolean')
}
