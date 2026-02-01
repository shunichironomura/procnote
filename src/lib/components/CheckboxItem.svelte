<script lang="ts">
	import type { CheckboxState } from '$lib/types';

	let {
		checkbox,
		disabled = false,
		ontoggle
	}: {
		checkbox: CheckboxState;
		disabled?: boolean;
		ontoggle: (text: string, checked: boolean) => void;
	} = $props();
</script>

<label class="checkbox-item" class:checked={checkbox.checked} class:disabled>
	<input
		type="checkbox"
		checked={checkbox.checked}
		{disabled}
		onchange={() => ontoggle(checkbox.text, !checkbox.checked)}
	/>
	<span class="checkbox-text">{checkbox.text}</span>
</label>

<style>
	.checkbox-item {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		padding: 6px 0;
		cursor: pointer;
		font-size: 13px;
	}

	.checkbox-item.disabled {
		cursor: default;
		opacity: 0.6;
	}

	.checkbox-item input[type='checkbox'] {
		margin-top: 2px;
		accent-color: #1a1a2e;
	}

	.checkbox-text {
		flex: 1;
	}

	.checked .checkbox-text {
		text-decoration: line-through;
		color: #888;
	}
</style>
