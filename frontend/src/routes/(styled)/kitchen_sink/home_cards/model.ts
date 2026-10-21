// What the home cards playground can switch between, shared by the control panel and the
// fake Home it drives in a frame.

export type Kind = 'script' | 'flow' | 'app' | 'raw_app' | 'agent'

export type Entry = {
	kind: Kind
	path: string
	/** Empty when the item has none: the card falls back to its path. */
	summary: string
	favorite: boolean
	/** Epoch ms of the last edit, for an edited item. */
	at?: number
}

export type Variant = 'current' | 'quiet' | 'tray' | 'columns' | 'tiles' | 'shelf' | 'inside'

export const VARIANTS: { value: Variant; label: string; description: string }[] = [
	{
		value: 'current',
		label: 'Current',
		description: 'What ships today: a framed card per item, time and menu always shown.'
	},
	{
		value: 'quiet',
		label: 'Quiet grid',
		description: 'The same grid with no frames: rows lift on hover, like the tree below.'
	},
	{
		value: 'tray',
		label: 'One tray',
		description: 'One recessed surface holds every item, so the group is a single shape.'
	},
	{
		value: 'columns',
		label: 'Two columns',
		description: 'Favorites and recent side by side, each a column that reads top to bottom.'
	},
	{
		value: 'tiles',
		label: 'Tiles',
		description: 'One row of larger tiles: titles wrap to two lines instead of truncating.'
	},
	{
		value: 'shelf',
		label: 'Shelf',
		description: 'Compact pills, like a bookmarks bar. Least space, no per-item menu.'
	},
	{
		value: 'inside',
		label: 'Inside the list',
		description: 'The shortcuts become the first band of the items card: one container.'
	}
]

export type Preset =
	| 'new'
	| 'first'
	| 'single'
	| 'typical'
	| 'favorites'
	| 'busy'
	| 'long'
	| 'paths'

export const PRESETS: { value: Preset; label: string }[] = [
	{ value: 'new', label: 'New workspace, nothing in it' },
	{ value: 'first', label: 'First steps: two items edited' },
	{ value: 'single', label: 'A single recent item' },
	{ value: 'typical', label: 'Typical: 2 favorites, 5 recent' },
	{ value: 'favorites', label: 'Favorites only' },
	{ value: 'busy', label: 'Busy: 10 favorites, 6 recent' },
	{ value: 'long', label: 'Long summaries' },
	{ value: 'paths', label: 'No summaries, long paths' }
]

export type Config = {
	variant: Variant
	preset: Preset
	/** The whole page, or every variant's section stacked for a side-by-side read. */
	view: 'page' | 'sections'
	/** Space between Home's sections. */
	spacing: 'tight' | 'airy'
	/** Kind icons in their colour, or grey until the item is hovered. */
	icons: 'colour' | 'muted'
	/** An item's ⋮ menu, cards and tree rows alike: always on screen, or on the item hovered. */
	menu: 'always' | 'hover'
	/** A recent card's edited time: always on screen, or on the card hovered. */
	time: 'always' | 'hover'
	/** A favorite's star in the shortcuts: always on screen, or on the card hovered. The tree's
	 * rows keep theirs, since there it is what tells a favorite from the rest. */
	stars: 'always' | 'hover'
	/** The items list in its framed card, or straight on the page. */
	listFrame: 'card' | 'flat'
	/** A folder or user row: its count on a second line, or beside its name. */
	treeRows: 'two' | 'one'
	chips: boolean
	composer: boolean
	label: boolean
	/** Home's column: today's 1120px, or narrower. */
	column: 'wide' | 'narrow'
	dark: boolean
}

export const DEFAULT_CONFIG: Config = {
	variant: 'current',
	preset: 'typical',
	view: 'page',
	spacing: 'tight',
	icons: 'colour',
	menu: 'always',
	time: 'always',
	stars: 'always',
	listFrame: 'card',
	treeRows: 'two',
	chips: true,
	composer: true,
	label: true,
	column: 'wide',
	dark: false
}

/** The page-level settings that calm Home whichever variant shows the cards. */
export const ZEN_PAGE: Partial<Config> = {
	spacing: 'airy',
	icons: 'muted',
	menu: 'hover',
	time: 'always',
	stars: 'hover',
	listFrame: 'card',
	treeRows: 'one',
	chips: false
}

/** Classes that hide an element until its `group` is hovered or holds focus, by opacity so
 * Tab still reaches it, and that keep it up while its own menu is open. A touch screen has no
 * hover to reveal it, so there it always shows. */
export const ON_HOVER =
	'opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 has-[[aria-expanded=true]]:opacity-100 [@media(hover:none)]:opacity-100'

/** A shortcut's star: an unset one always waits for hover, a set one too when `stars` says so. */
export function starClass(config: Config): string {
	return `inline-flex shrink-0 p-px text-yellow-500 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 ${
		config.stars === 'always' ? 'data-[starred]:opacity-100' : ''
	}`
}

const KEYS = Object.keys(DEFAULT_CONFIG) as (keyof Config)[]

/** Only what differs from the defaults, so a shared link stays short. */
export function encodeConfig(config: Config): URLSearchParams {
	const params = new URLSearchParams()
	for (const key of KEYS) {
		if (config[key] !== DEFAULT_CONFIG[key]) params.set(key, String(config[key]))
	}
	return params
}

export function decodeConfig(params: URLSearchParams): Config {
	const config: Record<string, unknown> = { ...DEFAULT_CONFIG }
	for (const key of KEYS) {
		const raw = params.get(key)
		if (raw == null) continue
		config[key] = typeof DEFAULT_CONFIG[key] === 'boolean' ? raw === 'true' : raw
	}
	return config as Config
}

/** What every variant of the shortcuts section is given. */
export type SectionProps = {
	entries: Entry[]
	config: Config
	now: number
	onToggleStar: (entry: Entry) => void
	onForget: (entry: Entry) => void
}

export const CONFIG_MESSAGE = 'home-cards:config'
export const KEY_MESSAGE = 'home-cards:key'
