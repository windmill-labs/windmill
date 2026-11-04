// Four changes far enough apart to land in separate hunks, plus two blocks in one hunk.
export const REVIEW_BEFORE = `import * as wmill from 'windmill-client'

type Invoice = {
	id: string
	amount: number
	currency: string
	paid: boolean
}

export async function main(customerId: string) {
	var invoices = await listInvoices(customerId)
	const unpaid = invoices.filter((i) => i.paid == false)

	let total = 0
	for (const invoice of unpaid) {
		total += invoice.amount
	}

	console.log('unpaid total', total)
	return { customerId, total, count: unpaid.length }
}

async function listInvoices(customerId: string): Promise<Invoice[]> {
	const stripe = await wmill.getResource('u/admin/stripe')
	const response = await fetch(\`https://api.stripe.com/v1/invoices?customer=\${customerId}\`, {
		headers: { Authorization: \`Bearer \${stripe.token}\` }
	})
	const body = await response.json()
	return body.data.map(toInvoice)
}

function toInvoice(raw: any): Invoice {
	return {
		id: raw.id,
		amount: raw.amount_due,
		currency: raw.currency,
		paid: raw.paid
	}
}

export function formatAmount(invoice: Invoice) {
	return invoice.amount / 100 + ' ' + invoice.currency
}
`

export const REVIEW_AFTER = `import * as wmill from 'windmill-client'

type Invoice = {
	id: string
	amount: number
	currency: string
	paid: boolean
	dueDate: Date
}

export async function main(customerId: string, overdueOnly = false) {
	const invoices = await listInvoices(customerId)
	const now = new Date()
	const unpaid = invoices.filter((i) => !i.paid && (!overdueOnly || i.dueDate < now))

	const total = unpaid.reduce((sum, invoice) => sum + invoice.amount, 0)

	return { customerId, total, count: unpaid.length }
}

async function listInvoices(customerId: string): Promise<Invoice[]> {
	const stripe = await wmill.getResource('f/billing/stripe')
	const response = await fetch(\`https://api.stripe.com/v1/invoices?customer=\${customerId}\`, {
		headers: { Authorization: \`Bearer \${stripe.token}\` }
	})
	if (!response.ok) {
		throw new Error(\`Stripe returned \${response.status}\`)
	}
	const body = await response.json()
	return body.data.map(toInvoice)
}

function toInvoice(raw: any): Invoice {
	return {
		id: raw.id,
		amount: raw.amount_due,
		currency: raw.currency,
		paid: raw.paid,
		dueDate: new Date(raw.due_date * 1000)
	}
}

export function formatAmount(invoice: Invoice) {
	return new Intl.NumberFormat('en', { style: 'currency', currency: invoice.currency }).format(
		invoice.amount / 100
	)
}
`
