<script lang="ts">
	let {
		notes,
		disabled = false,
		onadd
	}: {
		notes: string[];
		disabled?: boolean;
		onadd: (text: string) => void;
	} = $props();

	let noteText = $state('');

	function submit() {
		if (!noteText.trim()) return;
		onadd(noteText.trim());
		noteText = '';
	}
</script>

<div class="note-editor">
	{#if notes.length > 0}
		<ul class="note-list">
			{#each notes as note}
				<li class="note-item">{note}</li>
			{/each}
		</ul>
	{/if}
	{#if !disabled}
		<div class="note-input">
			<input
				type="text"
				bind:value={noteText}
				placeholder="Add a note..."
				onkeydown={(e) => { if (e.key === 'Enter') submit(); }}
			/>
			<button class="btn-add" onclick={submit} disabled={!noteText.trim()}>
				Add
			</button>
		</div>
	{/if}
</div>

<style>
	.note-list {
		list-style: none;
		margin: 0 0 8px;
		padding: 0;
	}

	.note-item {
		padding: 4px 0;
		font-size: 13px;
		color: #555;
		border-bottom: 1px solid #eee;
	}

	.note-item:last-child {
		border-bottom: none;
	}

	.note-input {
		display: flex;
		gap: 8px;
	}

	.note-input input {
		flex: 1;
		padding: 6px 10px;
		border: 1px solid #ccc;
		border-radius: 4px;
		font: inherit;
		font-size: 13px;
	}

	.note-input input:focus {
		outline: none;
		border-color: #1a1a2e;
		box-shadow: 0 0 0 2px rgba(26, 26, 46, 0.15);
	}

	.btn-add {
		padding: 6px 12px;
		background: #555;
		color: #fff;
		border: none;
		border-radius: 4px;
		font: inherit;
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-add:hover:not(:disabled) {
		background: #333;
	}

	.btn-add:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
</style>
