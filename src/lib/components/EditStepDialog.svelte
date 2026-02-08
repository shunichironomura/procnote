<script lang="ts">
    import type { StepSummary, StepContent } from "$lib/types";

    let {
        stepSummary,
        onconfirm,
        oncancel,
    }: {
        stepSummary: StepSummary;
        onconfirm: (stepHeading: string, content: StepContent[]) => void;
        oncancel: () => void;
    } = $props();

    // Extract existing prose text from content blocks.
    let initialProse = $derived(
        stepSummary.content
            .filter((b) => b.type === "Prose")
            .map((b) => (b as { type: "Prose"; text: string }).text)
            .join("\n\n"),
    );

    let description = $state("");

    // Initialize description from current prose on mount.
    $effect(() => {
        description = initialProse;
    });

    function submit() {
        // Rebuild content array: replace all Prose entries with a single new
        // one (if non-empty), keep Checkbox and InputBlock items in place.
        const newContent: StepContent[] = [];
        let proseInserted = false;

        for (const block of stepSummary.content) {
            if (block.type === "Prose") {
                // Insert new prose at the position of the first original Prose block.
                if (!proseInserted && description.trim()) {
                    newContent.push({ type: "Prose", text: description.trim() });
                    proseInserted = true;
                }
                // Skip remaining original Prose blocks (replaced by the single new one).
            } else if (block.type === "Checkbox") {
                newContent.push({
                    type: "Checkbox",
                    text: block.text,
                    checked: block.checked,
                });
            } else if (block.type === "InputBlock") {
                newContent.push({
                    type: "InputBlock",
                    inputs: block.inputs.map((i) => i.definition),
                });
            }
        }

        // If there were no Prose blocks originally but user typed something, append it.
        if (!proseInserted && description.trim()) {
            newContent.unshift({ type: "Prose", text: description.trim() });
        }

        onconfirm(stepSummary.heading, newContent);
    }
</script>

<div class="modal-backdrop" role="presentation" onclick={oncancel}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
        <h3>Edit Step Content</h3>
        <p class="hint">Edit the description for "{stepSummary.heading}".</p>
        <label class="field">
            <span class="field-label">Description</span>
            <!-- svelte-ignore a11y_autofocus -->
            <textarea
                bind:value={description}
                placeholder="Step description (Markdown supported)..."
                rows="6"
                autofocus
            ></textarea>
        </label>
        <div class="modal-actions">
            <button class="btn btn-secondary" onclick={oncancel}>Cancel</button>
            <button class="btn btn-primary" onclick={submit}>Save</button>
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

    .btn-primary {
        background: #1a1a2e;
        color: #fff;
    }

    .btn-primary:hover {
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
