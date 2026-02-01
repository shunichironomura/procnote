<script lang="ts">
	let {
		stepHeadings,
		onconfirm,
		oncancel
	}: {
		stepHeadings: string[];
		onconfirm: (heading: string, description?: string, afterStep?: string) => void;
		oncancel: () => void;
	} = $props();

	let heading = $state('');
	let description = $state('');
	let afterStep = $state('');

	function submit() {
		if (!heading.trim()) return;
		onconfirm(
			heading.trim(),
			description.trim() || undefined,
			afterStep || undefined
		);
	}
</script>

<div class="modal-backdrop" role="presentation" onclick={oncancel}>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="modal" onclick={(e) => e.stopPropagation()}>
		<h3>Add Step</h3>
		<p class="hint">Add a new step to the procedure execution.</p>
		<label class="field">
			<span class="field-label">Step Heading</span>
			<input
				type="text"
				bind:value={heading}
				placeholder="e.g. Additional Verification"
				autofocus
				onkeydown={(e) => { if (e.key === 'Enter') submit(); }}
			/>
		</label>
		<label class="field">
			<span class="field-label">Description (optional)</span>
			<textarea
				bind:value={description}
				placeholder="Describe what this step involves..."
				rows="2"
			></textarea>
		</label>
		<label class="field">
			<span class="field-label">Insert After (optional)</span>
			<select bind:value={afterStep}>
				<option value="">End of procedure</option>
				{#each stepHeadings as h}
					<option value={h}>{h}</option>
				{/each}
			</select>
		</label>
		<div class="modal-actions">
			<button class="btn btn-secondary" onclick={oncancel}>Cancel</button>
			<button class="btn btn-primary" onclick={submit} disabled={!heading.trim()}>
				Add Step
			</button>
		</div>
	</div>
</div>

<style>
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
		min-width: 400px;
		max-width: 520px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
	}

	.modal h3 {
		margin: 0 0 4px;
		font-size: 16px;
	}

	.hint {
		margin: 0 0 16px;
		font-size: 13px;
		color: #888;
	}

	.field {
		display: block;
		margin-bottom: 12px;
	}

	.field-label {
		display: block;
		font-size: 12px;
		font-weight: 600;
		margin-bottom: 4px;
		color: #555;
	}

	.field input,
	.field textarea,
	.field select {
		width: 100%;
		padding: 8px 10px;
		border: 1px solid #ccc;
		border-radius: 4px;
		font: inherit;
		font-size: 13px;
	}

	.field textarea {
		resize: vertical;
	}

	.field input:focus,
	.field textarea:focus,
	.field select:focus {
		outline: none;
		border-color: #1a1a2e;
		box-shadow: 0 0 0 2px rgba(26, 26, 46, 0.15);
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 16px;
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
