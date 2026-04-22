import React, { useState } from 'react'
import { backend } from 'wmill'

const CounterApp = () => {
	const [count, setCount] = useState(0)

	const increment = async () => {
		const newCount = await backend.incrementCounter({ currentCount: count })
		setCount(newCount)
	}

	const decrement = async () => {
		const newCount = await backend.decrementCounter({ currentCount: count })
		setCount(newCount)
	}

	return (
		<div className="p-4">
			<h1 className="text-2xl font-bold mb-4">Counter: {count}</h1>
			<div className="flex gap-2">
				<button
					onClick={decrement}
					className="px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600"
				>
					Decrement
				</button>
				<button
					onClick={increment}
					className="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600"
				>
					Increment
				</button>
			</div>
		</div>
	)
}

export default CounterApp
