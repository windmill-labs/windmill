import { describe, it, expect } from 'vitest'
import {
	addResourceTitle,
	resourceTypeDisplayName,
	sortResourceTypesByMatch
} from './resourceTypeDisplay'

// Descriptions abridged from the hub.
const TYPES = [
	{
		name: 'anthropic',
		description: 'Anthropic API key for the Claude models, or via Google Vertex'
	},
	{ name: 'gcal', description: 'Google OAuth token authorizing the Google Calendar API.' },
	{ name: 'googleai', description: 'API key for Google AI (Gemini), optionally on Vertex AI.' },
	{ name: 'gmail', description: 'Google OAuth token authorizing the Gmail API.' },
	{ name: 'mailchimp', description: 'Mailchimp API key.' },
	{ name: 'ms_sql_server', description: 'Connection settings for a SQL Server database.' },
	{ name: 'azure', description: 'Microsoft Azure tenant credentials.' }
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

	it('ranks a match on the display name above a description match', () => {
		// `ms_sql_server` displays as "Microsoft SQL Server"; azure only mentions Microsoft.
		const order = sorted('microsoft')
		expect(order.indexOf('ms_sql_server')).toBeLessThan(order.indexOf('azure'))
	})

	it('keeps the incoming order for an empty query', () => {
		expect(sorted('  ')).toEqual(TYPES.map((t) => t.name))
	})
})

describe('resourceTypeDisplayName', () => {
	it('takes the whole-name override when there is one', () => {
		expect(resourceTypeDisplayName('ms_sql_server')).toBe('Microsoft SQL Server')
	})

	it('drops the custom-type prefix and re-cases the rest', () => {
		expect(resourceTypeDisplayName('c_acme_api')).toBe('Acme API')
	})

	it('capitalizes anything the tables do not cover', () => {
		expect(resourceTypeDisplayName('stripe')).toBe('Stripe')
	})
})

describe('addResourceTitle', () => {
	it('names the picked type without an article', () => {
		expect(addResourceTitle('rest')).toBe('Add REST resource')
		expect(addResourceTitle('stripe')).toBe('Add Stripe resource')
	})

	it('falls back to the generic title before a type is picked', () => {
		expect(addResourceTitle(undefined)).toBe('Add a resource')
	})
})
