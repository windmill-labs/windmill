export { updatePolicy } from './components/apps/editor/appPolicy'
export { genWmillTs } from './components/raw_apps/utils'

export function capitalize(word: string): string {
	return word ? word.charAt(0).toUpperCase() + word.slice(1) : ''
}
