import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'

/** What Customize in the sidebar menu changes, per browser. Module-level so the
 * menu that flips a choice and the sidebar that reads it share one value. */
export const sidebarPrefs = {
	/** The starred items as a section of their own. */
	favorites: useLocalStorageValue('windmill_sidebar_show_favorites', false, 'boolean'),
	/** The reader's flow and agent chats. */
	aiChats: useLocalStorageValue('windmill_sidebar_show_ai_chats', true, 'boolean')
}

/** Set on the first click of the brand row: until then it also says it is Home, since the
 * brand alone does not tell a newcomer where it goes. */
export const homeRowLearned = useLocalStorageValue(
	'windmill_sidebar_home_row_learned',
	false,
	'boolean'
)
