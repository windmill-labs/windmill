/**
 * Resolve a file name (or relative path) to an icon by extension. Shared by the
 * raw-app file tree and the AI-chat file attachments so both stay consistent.
 */
import { File, ImageIcon } from 'lucide-svelte'
import TypeScript from '../common/languageIcons/TypeScript.svelte'
import JavaScriptIcon from './JavaScriptIcon.svelte'
import JsonIcon from './JsonIcon.svelte'
import ReactIcon from './ReactIcon.svelte'
import SvelteIcon from './SvelteIcon.svelte'
import VueIcon from './VueIcon.svelte'
import CssIcon from './CssIcon.svelte'
import SassIcon from './SassIcon.svelte'
import LessIcon from './LessIcon.svelte'
import HtmlIcon from './HtmlIcon.svelte'
import MarkdownIcon from './MarkdownIcon.svelte'
import YamlIcon from './YamlIcon.svelte'

export interface ResolvedFileIcon {
	icon: any
	className?: string
}

/** Lowercased extension of a file name or path (basename only); '' if none. */
export function getFileExtension(filename: string): string {
	const base = filename.split('/').pop() ?? filename
	const parts = base.split('.')
	return parts.length > 1 ? parts[parts.length - 1].toLowerCase() : ''
}

/** Icon (and optional color class) for a file, by extension. */
export function getFileIcon(filename: string): ResolvedFileIcon {
	switch (getFileExtension(filename)) {
		case 'json':
			return { icon: JsonIcon }
		case 'tsx':
		case 'jsx':
			return { icon: ReactIcon }
		case 'ts':
			return { icon: TypeScript }
		case 'js':
			return { icon: JavaScriptIcon }
		case 'svelte':
			return { icon: SvelteIcon }
		case 'vue':
			return { icon: VueIcon }
		case 'css':
			return { icon: CssIcon }
		case 'scss':
		case 'sass':
			return { icon: SassIcon }
		case 'less':
			return { icon: LessIcon }
		case 'png':
		case 'jpg':
		case 'jpeg':
		case 'gif':
		case 'svg':
		case 'webp':
		case 'ico':
			return { icon: ImageIcon, className: 'text-purple-500' }
		case 'html':
		case 'htm':
			return { icon: HtmlIcon }
		case 'md':
		case 'markdown':
			return { icon: MarkdownIcon }
		case 'yaml':
		case 'yml':
			return { icon: YamlIcon }
		default:
			return { icon: File, className: 'text-tertiary' }
	}
}
