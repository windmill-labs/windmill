import type { Option, OptionStyle } from './types'

// get the label key from an option object or the option itself if it's a string or number
export const get_label = (opt: Option) => {
	if (opt instanceof Object) {
		if (opt.label === undefined) {
			console.error(`MultiSelect option ${JSON.stringify(opt)} is an object but has no label key`)
		}
		return opt.label
	}
	return `${opt}`
}

// this function is used extract CSS strings from a {selected, option} style object to be used in the style attribute of the option
// if the style is a string, it will be returned as is
export function get_style(
	option: { style?: OptionStyle; [key: string]: unknown } | string | number,
	key: 'selected' | 'option' | null = null
) {
	let css_str = ``
	if (![`selected`, `option`, null].includes(key)) {
		console.error(`MultiSelect: Invalid key=${key} for get_style`)
	}
	if (typeof option == `object` && option.style) {
		if (typeof option.style == `string`) {
			css_str = option.style
		}
		if (typeof option.style == `object`) {
			if (key && key in option.style) return option.style[key] ?? ``
			else {
				console.error(`Invalid style object for option=${JSON.stringify(option)}`)
			}
		}
	}
	// ensure css_str ends with a semicolon
	if (css_str.trim() && !css_str.trim().endsWith(`;`)) css_str += `;`
	return css_str
}

// Firefox lacks support for scrollIntoViewIfNeeded (https://caniuse.com/scrollintoviewifneeded).
// See https://github.com/janosh/svelte-multiselect/issues/87
// Polyfill copied from
// https://github.com/nuxodin/lazyfill/blob/a8e63/polyfills/Element/prototype/scrollIntoViewIfNeeded.js
// exported for testing
export function scroll_into_view_if_needed_polyfill(elem: Element, centerIfNeeded: boolean = true) {
	const observer = new IntersectionObserver(function ([entry]) {
		const ratio = entry.intersectionRatio
		if (ratio < 1) {
			const place = ratio <= 0 && centerIfNeeded ? `center` : `nearest`
			elem.scrollIntoView({
				block: place,
				inline: place
			})
		}
		observer.disconnect()
	}, {
  root: null, // or specify a scrolling parent if needed
  rootMargin: '0px 1000px', // Essentially making horizontal checks irrelevant
  threshold: 0.1 // Adjust threshold to control when observer should trigger
})
	observer.observe(elem)

	return observer // return for testing
}
