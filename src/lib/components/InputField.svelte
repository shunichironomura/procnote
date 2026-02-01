<script lang="ts">
	import type { InputDefinition, InputState } from '$lib/types';

	let {
		definition,
		recorded,
		disabled = false,
		onrecord
	}: {
		definition: InputDefinition;
		recorded?: InputState;
		disabled?: boolean;
		onrecord: (label: string, value: string, unit?: string) => void;
	} = $props();

	let inputValue = $state(recorded?.value ?? '');

	function submit() {
		if (!inputValue.trim()) return;
		onrecord(definition.label, inputValue.trim(), definition.unit);
	}

	let expectedText = $derived.by(() => {
		if (!definition.expected) return null;
		if (typeof definition.expected === 'string') {
			return `Expected: ${definition.expected}`;
		}
		return `Expected: ${definition.expected.min} - ${definition.expected.max}${definition.unit ? ' ' + definition.unit : ''}`;
	});

	let isRecorded = $derived(!!recorded);
</script>

<div class="input-field" class:recorded={isRecorded}>
	<div class="input-header">
		<label class="input-label">{definition.label}</label>
		{#if expectedText}
			<span class="expected">{expectedText}</span>
		{/if}
	</div>
	<div class="input-row">
		{#if definition.input_type === 'selection'}
			<select
				bind:value={inputValue}
				disabled={disabled || isRecorded}
				onchange={submit}
			>
				<option value="">Select...</option>
				{#each definition.options as opt}
					<option value={opt}>{opt}</option>
				{/each}
			</select>
		{:else}
			<input
				type={definition.input_type === 'measurement' ? 'number' : 'text'}
				bind:value={inputValue}
				disabled={disabled || isRecorded}
				placeholder={definition.input_type === 'measurement' ? '0.0' : 'Enter value'}
				onkeydown={(e) => { if (e.key === 'Enter') submit(); }}
			/>
		{/if}
		{#if definition.unit}
			<span class="unit">{definition.unit}</span>
		{/if}
		{#if !isRecorded && definition.input_type !== 'selection'}
			<button class="btn-record" onclick={submit} disabled={disabled || !inputValue.trim()}>
				Record
			</button>
		{/if}
		{#if isRecorded}
			<span class="recorded-badge">Recorded</span>
		{/if}
	</div>
</div>

<style>
	.input-field {
		padding: 8px 12px;
		background: #f8f9fa;
		border: 1px solid #e0e0e0;
		border-radius: 4px;
	}

	.input-field.recorded {
		background: #e8f5e9;
		border-color: #c8e6c9;
	}

	.input-header {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		margin-bottom: 6px;
	}

	.input-label {
		font-size: 12px;
		font-weight: 600;
		color: #555;
	}

	.expected {
		font-size: 11px;
		color: #888;
	}

	.input-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.input-row input,
	.input-row select {
		flex: 1;
		padding: 6px 10px;
		border: 1px solid #ccc;
		border-radius: 4px;
		font: inherit;
		font-size: 13px;
	}

	.input-row input:focus,
	.input-row select:focus {
		outline: none;
		border-color: #1a1a2e;
		box-shadow: 0 0 0 2px rgba(26, 26, 46, 0.15);
	}

	.input-row input:disabled,
	.input-row select:disabled {
		background: #eee;
	}

	.unit {
		font-size: 12px;
		color: #666;
		white-space: nowrap;
	}

	.btn-record {
		padding: 6px 12px;
		background: #1a1a2e;
		color: #fff;
		border: none;
		border-radius: 4px;
		font: inherit;
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		white-space: nowrap;
	}

	.btn-record:hover:not(:disabled) {
		background: #16213e;
	}

	.btn-record:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.recorded-badge {
		font-size: 11px;
		font-weight: 600;
		color: #2e7d32;
		white-space: nowrap;
	}
</style>
