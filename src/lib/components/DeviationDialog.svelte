<script lang="ts">
    let {
        onconfirm,
        oncancel,
    }: {
        onconfirm: (description: string, justification: string) => void;
        oncancel: () => void;
    } = $props();

    let description = $state("");
    let justification = $state("");

    function submit() {
        if (!description.trim() || !justification.trim()) return;
        onconfirm(description.trim(), justification.trim());
    }
</script>

<div class="modal-backdrop" role="presentation" onclick={oncancel}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
        <h3>Record Deviation</h3>
        <p class="hint">Document any deviation from the nominal procedure.</p>
        <label class="field">
            <span class="field-label">Description</span>
            <textarea
                bind:value={description}
                placeholder="What deviated from the procedure?"
                rows="3"
            ></textarea>
        </label>
        <label class="field">
            <span class="field-label">Justification</span>
            <textarea
                bind:value={justification}
                placeholder="Why is this deviation acceptable?"
                rows="3"
            ></textarea>
        </label>
        <div class="modal-actions">
            <button class="btn btn-secondary" onclick={oncancel}>Cancel</button>
            <button
                class="btn btn-warn"
                onclick={submit}
                disabled={!description.trim() || !justification.trim()}
            >
                Record Deviation
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

    .field textarea {
        width: 100%;
        padding: 8px 10px;
        border: 1px solid #ccc;
        border-radius: 4px;
        font: inherit;
        font-size: 13px;
        resize: vertical;
    }

    .field textarea:focus {
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

    .btn-warn {
        background: #e65100;
        color: #fff;
    }

    .btn-warn:hover:not(:disabled) {
        background: #bf360c;
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
