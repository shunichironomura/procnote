<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>procnote</title>
</svelte:head>

<div class="app">
	<header class="app-header">
		<a href="/" class="app-title">procnote</a>
		<span class="app-subtitle">Procedure Execution</span>
	</header>
	<main class="app-main">
		{@render children()}
	</main>
</div>

<style>
	:global(*, *::before, *::after) {
		box-sizing: border-box;
	}

	:global(body) {
		margin: 0;
		font-family:
			-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell,
			sans-serif;
		background: #f5f5f5;
		color: #1a1a1a;
		font-size: 14px;
		line-height: 1.5;
	}

	:global(.timestamp) {
		font-size: 11px;
		color: #999;
		white-space: nowrap;
		flex-shrink: 0;
	}

	:global(.btn-delete) {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4px;
		background: none;
		color: #b71c1c;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		flex-shrink: 0;
		opacity: 0.5;
		transition:
			opacity 0.15s,
			background 0.15s;
	}

	:global(.btn-delete:hover) {
		opacity: 1;
		background: #fce4ec;
	}

	.app {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
	}

	.app-header {
		display: flex;
		align-items: baseline;
		gap: 12px;
		padding: 12px 24px;
		background: #1a1a2e;
		color: #e0e0e0;
		border-bottom: 2px solid #16213e;
	}

	.app-title {
		font-size: 18px;
		font-weight: 700;
		color: #e0e0e0;
		text-decoration: none;
		letter-spacing: -0.5px;
	}

	.app-title:hover {
		color: #fff;
	}

	.app-subtitle {
		font-size: 13px;
		color: #8888aa;
	}

	.app-main {
		flex: 1;
		padding: 24px;
		max-width: 1200px;
		width: 100%;
		margin: 0 auto;
	}
</style>
