<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import type { TemplateSummary, ExecutionSummary } from '$lib/types';
	import * as api from '$lib/api/commands';
	import { executionStore } from '$lib/stores/execution.svelte';
	import TemplateList from '$lib/components/TemplateList.svelte';

	let templates: TemplateSummary[] = $state([]);
	let executions: ExecutionSummary[] = $state([]);
	let loadingTemplates = $state(true);
	let loadingExecutions = $state(true);
	let error: string | null = $state(null);

	onMount(async () => {
		try {
			templates = await api.listTemplates();
		} catch (e) {
			error = String(e);
		} finally {
			loadingTemplates = false;
		}

		try {
			executions = await api.listExecutions();
		} catch (e) {
			// Non-critical: executions may not exist yet
		} finally {
			loadingExecutions = false;
		}
	});

	async function handleStart(template: TemplateSummary) {
		await executionStore.start(template.path);
		if (executionStore.summary) {
			goto(`/execution/${executionStore.summary.execution_id}`);
		}
	}

	function resumeExecution(exec: ExecutionSummary) {
		goto(`/execution/${exec.execution_id}`);
	}
</script>

<div class="home">
	<section class="section">
		<h2 class="section-title">Procedure Templates</h2>
		{#if loadingTemplates}
			<p class="loading">Loading templates...</p>
		{:else if error}
			<p class="error">{error}</p>
		{:else if templates.length === 0}
			<p class="empty">No procedure templates found. Place <code>.md</code> files in the procedures directory.</p>
		{:else}
			<TemplateList {templates} onstart={handleStart} />
		{/if}
	</section>

	{#if !loadingExecutions && executions.length > 0}
		<section class="section">
			<h2 class="section-title">Recent Executions</h2>
			<div class="execution-list">
				{#each executions as exec}
					<button class="execution-card" onclick={() => resumeExecution(exec)}>
						<div class="exec-header">
							<span class="exec-id">{exec.procedure_id}</span>
							<span class="exec-status" class:status-active={exec.status === 'active'} class:status-pass={exec.status === 'pass'} class:status-fail={exec.status === 'fail'} class:status-aborted={exec.status === 'aborted'}>
								{exec.status}
							</span>
						</div>
						<div class="exec-meta">
							<span>v{exec.procedure_version}</span>
						</div>
					</button>
				{/each}
			</div>
		</section>
	{/if}
</div>

<style>
	.home {
		display: flex;
		flex-direction: column;
		gap: 32px;
	}

	.section-title {
		font-size: 16px;
		font-weight: 600;
		margin: 0 0 16px;
		color: #333;
	}

	.loading, .empty {
		color: #666;
		font-style: italic;
	}

	.error {
		color: #c0392b;
	}

	.execution-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.execution-card {
		display: block;
		width: 100%;
		text-align: left;
		padding: 12px 16px;
		background: #fff;
		border: 1px solid #ddd;
		border-radius: 6px;
		cursor: pointer;
		font: inherit;
	}

	.execution-card:hover {
		border-color: #aaa;
		background: #fafafa;
	}

	.exec-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 4px;
	}

	.exec-id {
		font-weight: 600;
	}

	.exec-status {
		font-size: 12px;
		font-weight: 600;
		padding: 2px 8px;
		border-radius: 10px;
		text-transform: uppercase;
		background: #eee;
		color: #666;
	}

	.status-active {
		background: #e8f5e9;
		color: #2e7d32;
	}

	.status-pass {
		background: #e0f2f1;
		color: #00695c;
	}

	.status-fail {
		background: #fce4ec;
		color: #c62828;
	}

	.status-aborted {
		background: #fff3e0;
		color: #e65100;
	}

	.exec-meta {
		display: flex;
		gap: 16px;
		font-size: 12px;
		color: #888;
	}
</style>
