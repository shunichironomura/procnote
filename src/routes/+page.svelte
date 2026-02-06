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

	let operatorName = $state('');
	let startingTemplate: TemplateSummary | null = $state(null);

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
		if (!operatorName.trim()) {
			startingTemplate = template;
			return;
		}
		await startExecution(template);
	}

	async function confirmStart() {
		if (!startingTemplate || !operatorName.trim()) return;
		await startExecution(startingTemplate);
		startingTemplate = null;
	}

	async function startExecution(template: TemplateSummary) {
		await executionStore.start(template.path, operatorName.trim());
		if (executionStore.summary) {
			goto(`/execution/${executionStore.summary.execution_id}`);
		}
	}

	function cancelStart() {
		startingTemplate = null;
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
							<span>Operator: {exec.operator}</span>
						</div>
					</button>
				{/each}
			</div>
		</section>
	{/if}
</div>

{#if startingTemplate}
	<div class="modal-backdrop" role="presentation" onclick={cancelStart}>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h3>Start Execution</h3>
			<p>Procedure: <strong>{startingTemplate.title}</strong></p>
			<label class="field">
				<span class="field-label">Operator Name</span>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					type="text"
					bind:value={operatorName}
					placeholder="Enter your name"
					autofocus
					onkeydown={(e) => { if (e.key === 'Enter') confirmStart(); }}
				/>
			</label>
			{#if executionStore.error}
				<p class="error">{executionStore.error}</p>
			{/if}
			<div class="modal-actions">
				<button class="btn btn-secondary" onclick={cancelStart}>Cancel</button>
				<button class="btn btn-primary" onclick={confirmStart} disabled={!operatorName.trim() || executionStore.loading}>
					{executionStore.loading ? 'Starting...' : 'Start'}
				</button>
			</div>
		</div>
	</div>
{/if}

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

	/* Modal */
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.4);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.modal {
		background: #fff;
		border-radius: 8px;
		padding: 24px;
		min-width: 360px;
		max-width: 480px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
	}

	.modal h3 {
		margin: 0 0 12px;
		font-size: 16px;
	}

	.modal p {
		margin: 0 0 16px;
		font-size: 13px;
		color: #555;
	}

	.field {
		display: block;
		margin-bottom: 16px;
	}

	.field-label {
		display: block;
		font-size: 12px;
		font-weight: 600;
		margin-bottom: 4px;
		color: #555;
	}

	.field input {
		width: 100%;
		padding: 8px 12px;
		border: 1px solid #ccc;
		border-radius: 4px;
		font: inherit;
		font-size: 14px;
	}

	.field input:focus {
		outline: none;
		border-color: #1a1a2e;
		box-shadow: 0 0 0 2px rgba(26, 26, 46, 0.15);
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}

	.btn {
		padding: 8px 16px;
		border-radius: 4px;
		font: inherit;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		border: 1px solid transparent;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-primary {
		background: #1a1a2e;
		color: #fff;
	}

	.btn-primary:hover:not(:disabled) {
		background: #16213e;
	}

	.btn-secondary {
		background: #fff;
		color: #333;
		border-color: #ccc;
	}

	.btn-secondary:hover {
		background: #f5f5f5;
	}
</style>
