<script lang="ts">
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { executionStore } from "$lib/stores/execution.svelte";
    import type { ExecutionAction } from "$lib/types";
    import StepCard from "$lib/components/StepCard.svelte";
    import AddStepDialog from "$lib/components/AddStepDialog.svelte";

    let showAddStepDialog = $state(false);
    let showCompleteDialog = $state(false);
    let showAbortDialog = $state(false);
    let abortReason = $state("");

    let executionId = $derived(page.params.id ?? "");

    onMount(async () => {
        if (executionId) {
            await executionStore.load(executionId);
        }
    });

    let summary = $derived(executionStore.summary);
    let isActive = $derived(summary?.status === "active");
    let isFinished = $derived(executionStore.isFinished);

    let completedSteps = $derived(
        summary?.steps.filter((s) => s.status === "completed").length ?? 0,
    );
    let totalSteps = $derived(summary?.steps.length ?? 0);

    let stepHeadings = $derived(summary?.steps.map((s) => s.heading) ?? []);

    async function handleAction(action: Record<string, unknown>) {
        await executionStore.act(action as ExecutionAction);
    }

    async function addStep(
        heading: string,
        description?: string,
        afterStep?: string,
    ) {
        await executionStore.act({
            action: "add_step",
            heading,
            description,
            after_step: afterStep,
        });
        showAddStepDialog = false;
    }

    async function completeExecution(status: "pass" | "fail") {
        await executionStore.act({ action: "complete", status });
        showCompleteDialog = false;
    }

    async function abortExecution() {
        if (!abortReason.trim()) return;
        await executionStore.act({
            action: "abort",
            reason: abortReason.trim(),
        });
        showAbortDialog = false;
        abortReason = "";
    }

    function goHome() {
        executionStore.reset();
        goto("/");
    }
</script>

<div class="execution-page">
    {#if executionStore.loading && !summary}
        <p class="loading">Loading execution...</p>
    {:else if executionStore.error && !summary}
        <div class="error-panel">
            <p>{executionStore.error}</p>
            <button class="btn btn-secondary" onclick={goHome}
                >Back to Home</button
            >
        </div>
    {:else if summary}
        <div class="execution-header">
            <div class="header-left">
                <button class="btn-back" onclick={goHome}>&larr; Back</button>
                <div class="header-info">
                    <h2 class="procedure-title">{summary.procedure_id}</h2>
                    <span class="procedure-meta">
                        v{summary.procedure_version} &middot; Operator: {summary.operator}
                    </span>
                </div>
            </div>
            <div class="header-right">
                <span
                    class="execution-status"
                    class:status-active={isActive}
                    class:status-pass={summary.status === "pass"}
                    class:status-fail={summary.status === "fail"}
                    class:status-aborted={summary.status === "aborted"}
                >
                    {summary.status}
                </span>
                <span class="progress">{completedSteps}/{totalSteps} steps</span
                >
            </div>
        </div>

        {#if executionStore.error}
            <div class="error-bar">{executionStore.error}</div>
        {/if}

        {#if isActive}
            <div class="toolbar">
                <button
                    class="btn btn-secondary"
                    onclick={() => (showAddStepDialog = true)}
                >
                    + Add Step
                </button>
                <div class="toolbar-spacer"></div>
                <button
                    class="btn btn-success"
                    onclick={() => (showCompleteDialog = true)}
                >
                    Complete
                </button>
                <button
                    class="btn btn-danger"
                    onclick={() => (showAbortDialog = true)}
                >
                    Abort
                </button>
            </div>
        {/if}

        {#if isFinished}
            <div
                class="finish-banner"
                class:pass={summary.status === "pass"}
                class:fail={summary.status === "fail"}
                class:aborted={summary.status === "aborted"}
            >
                Execution {summary.status === "pass"
                    ? "passed"
                    : summary.status === "fail"
                      ? "failed"
                      : "aborted"}
                &mdash; {completedSteps}/{totalSteps} steps completed
            </div>
        {/if}

        <div class="steps">
            {#each summary.steps as stepSummary}
                <StepCard
                    {stepSummary}
                    executionActive={isActive}
                    onaction={handleAction}
                />
            {/each}
        </div>
    {/if}
</div>

{#if showAddStepDialog}
    <AddStepDialog
        {stepHeadings}
        onconfirm={addStep}
        oncancel={() => (showAddStepDialog = false)}
    />
{/if}

{#if showCompleteDialog}
    <div
        class="modal-backdrop"
        role="presentation"
        onclick={() => (showCompleteDialog = false)}
    >
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="modal" onclick={(e) => e.stopPropagation()}>
            <h3>Complete Execution</h3>
            <p>Mark this execution as:</p>
            <div class="modal-actions">
                <button
                    class="btn btn-secondary"
                    onclick={() => (showCompleteDialog = false)}>Cancel</button
                >
                <button
                    class="btn btn-danger"
                    onclick={() => completeExecution("fail")}>Fail</button
                >
                <button
                    class="btn btn-success"
                    onclick={() => completeExecution("pass")}>Pass</button
                >
            </div>
        </div>
    </div>
{/if}

{#if showAbortDialog}
    <div
        class="modal-backdrop"
        role="presentation"
        onclick={() => {
            showAbortDialog = false;
            abortReason = "";
        }}
    >
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="modal" onclick={(e) => e.stopPropagation()}>
            <h3>Abort Execution</h3>
            <p>This will permanently mark the execution as aborted.</p>
            <label class="field">
                <span class="field-label">Reason</span>
                <textarea
                    bind:value={abortReason}
                    placeholder="Why is the execution being aborted?"
                    rows="3"
                ></textarea>
            </label>
            <div class="modal-actions">
                <button
                    class="btn btn-secondary"
                    onclick={() => {
                        showAbortDialog = false;
                        abortReason = "";
                    }}>Cancel</button
                >
                <button
                    class="btn btn-danger"
                    onclick={abortExecution}
                    disabled={!abortReason.trim()}>Abort</button
                >
            </div>
        </div>
    </div>
{/if}

<style>
    .execution-page {
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .loading {
        color: #666;
        font-style: italic;
    }

    .error-panel {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 12px;
        color: #c0392b;
    }

    .error-bar {
        padding: 8px 12px;
        background: #fce4ec;
        color: #c62828;
        border-radius: 4px;
        font-size: 13px;
    }

    .execution-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 16px;
    }

    .header-left {
        display: flex;
        align-items: flex-start;
        gap: 12px;
    }

    .btn-back {
        padding: 4px 8px;
        background: none;
        border: 1px solid #ccc;
        border-radius: 4px;
        font: inherit;
        font-size: 13px;
        cursor: pointer;
        color: #555;
        margin-top: 2px;
    }

    .btn-back:hover {
        background: #f0f0f0;
    }

    .header-info {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .procedure-title {
        margin: 0;
        font-size: 18px;
        font-weight: 700;
    }

    .procedure-meta {
        font-size: 13px;
        color: #888;
    }

    .header-right {
        display: flex;
        align-items: center;
        gap: 12px;
        flex-shrink: 0;
    }

    .execution-status {
        font-size: 12px;
        font-weight: 600;
        padding: 3px 10px;
        border-radius: 12px;
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

    .progress {
        font-size: 13px;
        color: #888;
    }

    .toolbar {
        display: flex;
        gap: 8px;
        padding: 12px 0;
        border-bottom: 1px solid #eee;
    }

    .toolbar-spacer {
        flex: 1;
    }

    .finish-banner {
        padding: 12px 16px;
        border-radius: 6px;
        font-weight: 600;
        font-size: 14px;
        text-align: center;
    }

    .finish-banner.pass {
        background: #e0f2f1;
        color: #00695c;
    }

    .finish-banner.fail {
        background: #fce4ec;
        color: #c62828;
    }

    .finish-banner.aborted {
        background: #fff3e0;
        color: #e65100;
    }

    .steps {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    /* Buttons */
    .btn {
        padding: 6px 16px;
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

    .btn-secondary {
        background: #fff;
        color: #333;
        border-color: #ccc;
    }

    .btn-secondary:hover {
        background: #f5f5f5;
    }

    .btn-success {
        background: #2e7d32;
        color: #fff;
    }

    .btn-success:hover {
        background: #1b5e20;
    }

    .btn-danger {
        background: #c62828;
        color: #fff;
    }

    .btn-danger:hover:not(:disabled) {
        background: #b71c1c;
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
        margin: 0 0 8px;
        font-size: 16px;
    }

    .modal p {
        margin: 0 0 16px;
        font-size: 13px;
        color: #555;
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
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
</style>
