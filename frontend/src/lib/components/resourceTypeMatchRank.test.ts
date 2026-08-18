import { describe, it, expect } from 'vitest'
import { sortResourceTypesByMatch } from './resourceTypeDisplay'

// Descriptions abridged from the hub.
const TYPES = [
	{
		name: 'anthropic',
		description: 'Anthropic API key for the Claude models, or via Google Vertex'
	},
	{ name: 'gcal', description: 'Google OAuth token authorizing the Google Calendar API.' },
	{ name: 'googleai', description: 'API key for Google AI (Gemini), optionally on Vertex AI.' },
	{ name: 'gmail', description: 'Google OAuth token authorizing the Gmail API.' },
	{ name: 'mailchimp', description: 'Mailchimp API key.' }
]

const sorted = (query: string) =>
	sortResourceTypesByMatch(
		TYPES,
		query,
		(t) => t.name,
		(t) => t.description
	).map((t) => t.name)

describe('sortResourceTypesByMatch', () => {
	it('ranks a match on the type name above any description match', () => {
		expect(sorted('google')[0]).toBe('googleai')
	})

	it('ranks a description opening with the query above one mentioning it in passing', () => {
		const order = sorted('google')
		expect(order.indexOf('gcal')).toBeLessThan(order.indexOf('anthropic'))
	})

	it('ranks a name the query starts above one where it appears mid-word', () => {
		const order = sorted('mail')
		expect(order.indexOf('mailchimp')).toBeLessThan(order.indexOf('gmail'))
	})

	it('keeps the incoming order for an empty query', () => {
		expect(sorted('  ')).toEqual(TYPES.map((t) => t.name))
	})
})
