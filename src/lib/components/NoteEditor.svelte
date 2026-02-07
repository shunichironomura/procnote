<script lang="ts">
    let {
        notes,
        disabled = false,
        onadd,
        onrevert,
    }: {
        notes: string[];
        disabled?: boolean;
        onadd: (text: string) => void;
        onrevert?: (noteIndex: number) => void;
    } = $props();

    let noteText = $state("");

    function submit() {
        if (!noteText.trim()) return;
        onadd(noteText.trim());
        noteText = "";
    }
</script>

<div class="note-editor">
    {#if notes.length > 0}
        <ul class="note-list">
            {#each notes as note, i}
                <li class="note-item">
                    <span class="note-text">{note}</span>
                    {#if onrevert}
                        <button class="btn-delete" title="Delete note" onclick={() => onrevert(i)}>
                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
                        </button>
                    {/if}
                </li>
            {/each}
        </ul>
    {/if}
    {#if !disabled}
        <div class="note-input">
            <input
                type="text"
                bind:value={noteText}
                placeholder="Add a note..."
                onkeydown={(e) => {
                    if (e.key === "Enter") submit();
                }}
            />
            <button
                class="btn-add"
                onclick={submit}
                disabled={!noteText.trim()}
            >
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
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 4px 0;
        font-size: 13px;
        color: #555;
        border-bottom: 1px solid #eee;
    }

    .note-item:last-child {
        border-bottom: none;
    }

    .note-text {
        flex: 1;
    }

    .btn-delete {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 4px;
        background: none;
        color: #b71c1c;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        flex-shrink: 0;
        opacity: 0.5;
        transition: opacity 0.15s, background 0.15s;
    }

    .btn-delete:hover {
        opacity: 1;
        background: #fce4ec;
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
