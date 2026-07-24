import { describe, expect, it } from 'vitest'
import { rewriteCssUrls } from './rawAppSnapshot'

describe('rewriteCssUrls', () => {
	it('resolves relative references against the stylesheet, not the document', () => {
		const css = rewriteCssUrls(
			`.a { background: url(img/bg.png) } .b { background: url("../fonts/f.woff2") }`,
			'http://localhost:3000/api/w/demo/apps_u/get_data/v/abc.css'
		)
		expect(css).toBe(
			`.a { background: url(http://localhost:3000/api/w/demo/apps_u/get_data/v/img/bg.png) } ` +
				`.b { background: url("http://localhost:3000/api/w/demo/apps_u/get_data/fonts/f.woff2") }`
		)
	})

	it('leaves references that are already resolvable alone', () => {
		const css = `.a { background: url(data:image/gif;base64,R0lGOD) } .b { background: url(https://cdn/x.png) }`
		expect(rewriteCssUrls(css, 'http://localhost:3000/app.css')).toBe(css)
	})
})
