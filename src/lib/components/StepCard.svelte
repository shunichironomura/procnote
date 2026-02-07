<script lang="ts">
    import type { StepSummary, EventHistoryEntry } from "$lib/types";
    import CheckboxItem from "./CheckboxItem.svelte";
    import InputField from "./InputField.svelte";
    import NoteEditor from "./NoteEditor.svelte";

    let {
        stepSummary,
        executionActive = false,
        revertibleEvents = [],
        onaction,
    }: {
        stepSummary: StepSummary;
        executionActive?: boolean;
        revertibleEvents?: EventHistoryEntry[];
        onaction: (action: Record<string, unknown>) => void;
    } = $props();

    let isActive = $derived(stepSummary.status === "active");
    let isCompleted = $derived(stepSummary.status === "completed");
    let isSkipped = $derived(stepSummary.status === "skipped");
    let isPending = $derived(stepSummary.status === "pending");
    let isInteractable = $derived(isActive && executionActive);

    let showSkipDialog = $state(false);
    let skipReason = $state("");

    // Find the most recent revertible step-status event (complete/skip/start).
    let revertibleStatusEvent = $derived(
        revertibleEvents
            .filter(
                (e) =>
                    e.event_type === "step_completed" ||
                    e.event_type === "step_skipped" ||
                    e.event_type === "step_started",
            )
            .at(-1),
    );

    // Build a map of input label -> most recent revertible input_recorded event.
    // Description format: "Recorded <label> = <value> in <step_heading>"
    let revertibleInputEvents = $derived.by(() => {
        const map = new Map<string, EventHistoryEntry>();
        for (const e of revertibleEvents) {
            if (e.event_type === "input_recorded") {
                // Extract label from description: "Recorded <label> = ..."
                const match = e.description.match(/^Recorded (.+?) = /);
                if (match) {
                    map.set(match[1], e);
                }
            }
        }
        return map;
    });

    function startStep() {
        onaction({ action: "start_step", step_heading: stepSummary.heading });
    }

    function completeStep() {
        onaction({
            action: "complete_step",
            step_heading: stepSummary.heading,
        });
    }

    function confirmSkip() {
        if (!skipReason.trim()) return;
        onaction({
            action: "skip_step",
            step_heading: stepSummary.heading,
            reason: skipReason.trim(),
        });
        showSkipDialog = false;
        skipReason = "";
    }

    function toggleCheckbox(text: string, checked: boolean) {
        onaction({
            action: "toggle_checkbox",
            step_heading: stepSummary.heading,
            text,
            checked,
        });
    }

    function recordInput(label: string, value: string, unit?: string) {
        onaction({
            action: "record_input",
            step_heading: stepSummary.heading,
            label,
            value,
            unit,
        });
    }

    function addNote(text: string) {
        onaction({
            action: "add_note",
            text,
            step_heading: stepSummary.heading,
        });
    }
</script>

<div
    class="step-card"
    class:active={isActive}
    class:completed={isCompleted}
    class:skipped={isSkipped}
    class:pending={isPending}
>
    <div class="step-header">
        <div class="step-status-indicator"></div>
        <h3 class="step-heading">{stepSummary.heading}</h3>
        <span class="step-status-badge">{stepSummary.status}</span>
    </div>

    {#if stepSummary.description}
        <p class="step-description">{stepSummary.description}</p>
    {/if}

    {#if stepSummary.checkboxes.length > 0}
        <div class="step-section">
            {#each stepSummary.checkboxes as checkbox}
                <CheckboxItem
                    {checkbox}
                    disabled={!isInteractable}
                    ontoggle={toggleCheckbox}
                />
            {/each}
        </div>
    {/if}

    {#if stepSummary.input_definitions.length > 0}
        <div class="step-section">
            {#each stepSummary.input_definitions as defn}
                {@const inputEvent = revertibleInputEvents.get(defn.label)}
                <InputField
                    definition={defn}
                    recorded={stepSummary.inputs.find(
                        (i) => i.label === defn.label,
                    )}
                    disabled={!isInteractable}
                    onrecord={recordInput}
                    onrevert={inputEvent && executionActive
                        ? () =>
                              onaction({
                                  action: "revert_event",
                                  event_index: inputEvent.index,
                                  reason: "Reverted by operator",
                              })
                        : undefined}
                />
            {/each}
        </div>
    {/if}

    <div class="step-section">
        <NoteEditor
            notes={stepSummary.notes}
            disabled={!isInteractable}
            onadd={addNote}
        />
    </div>

    {#if executionActive}
        <div class="step-actions">
            {#if isPending}
                <button class="btn btn-primary" onclick={startStep}
                    >Start Step</button
                >
                <button
                    class="btn btn-muted"
                    onclick={() => (showSkipDialog = true)}>Skip</button
                >
            {:else if isActive}
                <button class="btn btn-primary" onclick={completeStep}
                    >Complete Step</button
                >
                <button
                    class="btn btn-muted"
                    onclick={() => (showSkipDialog = true)}>Skip</button
                >
            {/if}
            {#if revertibleStatusEvent && (isCompleted || isSkipped)}
                <button
                    class="btn btn-undo"
                    onclick={() =>
                        onaction({
                            action: "revert_event",
                            event_index: revertibleStatusEvent.index,
                            reason: "Reverted by operator",
                        })}
                >
                    Undo {isCompleted ? "Complete" : "Skip"}
                </button>
            {/if}
        </div>
    {/if}

    {#if showSkipDialog}
        <div class="skip-dialog">
            <label class="field">
                <span class="field-label">Reason for skipping</span>
                <!-- svelte-ignore a11y_autofocus -->
                <input
                    type="text"
                    bind:value={skipReason}
                    placeholder="Enter reason..."
                    autofocus
                    onkeydown={(e) => {
                        if (e.key === "Enter") confirmSkip();
                    }}
                />
            </label>
            <div class="skip-actions">
                <button
                    class="btn btn-muted"
                    onclick={() => {
                        showSkipDialog = false;
                        skipReason = "";
                    }}>Cancel</button
                >
                <button
                    class="btn btn-warn"
                    onclick={confirmSkip}
                    disabled={!skipReason.trim()}>Skip Step</button
                >
            </div>
        </div>
    {/if}
</div>

<style>
    .step-card {
        background: #fff;
        border: 1px solid #ddd;
        border-radius: 8px;
        padding: 16px;
        transition: border-color 0.15s;
    }

    .step-card.active {
        border-color: #1a1a2e;
        box-shadow: 0 0 0 1px #1a1a2e;
    }

    .step-card.completed {
        border-color: #c8e6c9;
        background: #fafff9;
    }

    .step-card.skipped {
        border-color: #ffe0b2;
        background: #fffdf5;
        opacity: 0.8;
    }

    .step-card.pending {
        opacity: 0.7;
    }

    .step-header {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-bottom: 12px;
    }

    .step-status-indicator {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        background: #ccc;
        flex-shrink: 0;
    }

    .active .step-status-indicator {
        background: #1a1a2e;
        box-shadow: 0 0 0 3px rgba(26, 26, 46, 0.2);
    }

    .completed .step-status-indicator {
        background: #2e7d32;
    }

    .skipped .step-status-indicator {
        background: #e65100;
    }

    .step-heading {
        flex: 1;
        margin: 0;
        font-size: 15px;
        font-weight: 600;
    }

    .step-description {
        margin: 0 0 4px;
        font-size: 14px;
        color: #444;
        line-height: 1.5;
        white-space: pre-line;
    }

    .step-status-badge {
        font-size: 11px;
        font-weight: 600;
        text-transform: uppercase;
        color: #888;
    }

    .active .step-status-badge {
        color: #1a1a2e;
    }

    .completed .step-status-badge {
        color: #2e7d32;
    }

    .skipped .step-status-badge {
        color: #e65100;
    }

    .step-section {
        margin-top: 12px;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .step-actions {
        display: flex;
        gap: 8px;
        margin-top: 16px;
        padding-top: 12px;
        border-top: 1px solid #eee;
    }

    .btn {
        padding: 6px 16px;
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

    .btn-muted {
        background: #fff;
        color: #666;
        border-color: #ccc;
    }

    .btn-muted:hover {
        background: #f5f5f5;
    }

    .btn-undo {
        background: #fff;
        color: #6a1b9a;
        border-color: #ce93d8;
        margin-left: auto;
    }

    .btn-undo:hover {
        background: #f3e5f5;
    }

    .btn-warn {
        background: #e65100;
        color: #fff;
    }

    .btn-warn:hover:not(:disabled) {
        background: #bf360c;
    }

    .btn-warn:disabled {
        opacity: 0.4;
        cursor: not-allowed;
    }

    .skip-dialog {
        margin-top: 12px;
        padding: 12px;
        background: #fff8e1;
        border: 1px solid #ffe082;
        border-radius: 4px;
    }

    .field {
        display: block;
        margin-bottom: 8px;
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
        padding: 6px 10px;
        border: 1px solid #ccc;
        border-radius: 4px;
        font: inherit;
        font-size: 13px;
    }

    .field input:focus {
        outline: none;
        border-color: #1a1a2e;
        box-shadow: 0 0 0 2px rgba(26, 26, 46, 0.15);
    }

    .skip-actions {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
    }
</style>
