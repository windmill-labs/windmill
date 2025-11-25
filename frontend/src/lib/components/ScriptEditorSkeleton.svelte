<div class="flex flex-col h-screen bg-surface">
	<!-- Top bar -->
	<div class="h-12 px-4 border-b flex items-center gap-4">
		<div class="w-8 h-8 rounded animate-skeleton dark:bg-frost-900/50"></div>
		<div class="flex-1 h-6 max-w-md rounded animate-skeleton dark:bg-frost-900/50"></div>
	</div>

	<!-- Editor bar -->
	<div class="border-b shadow-sm px-4 py-2 flex gap-2">
		<div class="w-20 h-6 rounded animate-skeleton dark:bg-frost-900/50"></div>
		<div class="w-20 h-6 rounded animate-skeleton dark:bg-frost-900/50"></div>
		<div class="w-20 h-6 rounded animate-skeleton dark:bg-frost-900/50"></div>
		<div class="flex-1"></div>
		<div class="w-24 h-6 rounded animate-skeleton dark:bg-frost-900/50"></div>
	</div>

	<!-- Main content area -->
	<div class="flex flex-1 overflow-hidden">
		<!-- Left panel - Editor -->
		<div class="flex-1 min-w-0 border-r bg-surface-secondary">
			<div class="w-full h-full p-4">
				<div class="w-32 h-4 mb-3 rounded animate-skeleton dark:bg-frost-900/50"></div>
				<div class="w-48 h-4 mb-3 rounded animate-skeleton dark:bg-frost-900/50"></div>
				<div class="w-40 h-4 mb-3 rounded animate-skeleton dark:bg-frost-900/50"></div>
				<div class="w-56 h-4 mb-3 rounded animate-skeleton dark:bg-frost-900/50"></div>
			</div>
		</div>

		<!-- Right panel - Test/Preview with horizontal split -->
		<div class="w-2/5 min-w-0 flex flex-col bg-surface">
			<!-- Top section - test inputs -->
			<div class="flex-1 border-b flex flex-col">
				<div class="h-10 border-b px-4 flex items-center">
					<div class="w-24 h-4 rounded animate-skeleton dark:bg-frost-900/50"></div>
				</div>
				<div class="flex-1 p-4 space-y-3">
					<div class="w-full h-8 rounded animate-skeleton dark:bg-frost-900/50"></div>
					<div class="w-full h-8 rounded animate-skeleton dark:bg-frost-900/50"></div>
					<div class="w-32 h-8 rounded animate-skeleton dark:bg-frost-900/50"></div>
				</div>
			</div>

			<!-- Bottom section - results -->
			<div class="flex-1 flex flex-col">
				<div class="h-10 border-b px-4 flex items-center">
					<div class="w-24 h-4 rounded animate-skeleton dark:bg-frost-900/50"></div>
				</div>
				<div class="flex-1 p-4">
					<div class="w-full h-4 mb-2 rounded animate-skeleton dark:bg-frost-900/50"></div>
					<div class="w-3/4 h-4 rounded animate-skeleton dark:bg-frost-900/50"></div>
				</div>
			</div>
		</div>
	</div>
</div>
