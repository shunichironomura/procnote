<script lang="ts">
    import type { InputDefinition, InputState } from "$lib/types";
    import { formatTimestamp } from "$lib/utils/format";
    import { inferContentType } from "$lib/utils/mime";
    import { open } from "@tauri-apps/plugin-dialog";
    import TrashIcon from "./TrashIcon.svelte";

    let {
        definition,
        recorded,
        disabled = false,
        onattach,
        onrevert,
    }: {
        definition: InputDefinition;
        recorded?: InputState;
        disabled?: boolean;
        onattach: (filename: string, path: string, contentType: string) => void;
        onrevert?: () => void;
    } = $props();

    let selectedPath = $state<string | null>(null);
    let selectedFilename = $state<string | null>(null);

    let isRecorded = $derived(!!recorded);

    async function pickFile() {
        const result = await open({
            multiple: false,
            directory: false,
            title: definition.label,
        });
        if (result) {
            selectedPath = result;
            selectedFilename = result.split(/[/\\]/).pop() ?? result;
        }
    }

    function confirm() {
        if (selectedPath && selectedFilename) {
            const contentType = inferContentType(selectedFilename);
            onattach(selectedFilename, selectedPath, contentType);
            selectedPath = null;
            selectedFilename = null;
        }
    }

    function clear() {
        selectedPath = null;
        selectedFilename = null;
    }
</script>

<div class="input-field" class:recorded={isRecorded}>
    <div class="input-header">
        <span class="input-label">{definition.label}</span>
    </div>
    <div class="input-row">
        {#if isRecorded}
            <span class="filename">{recorded?.value}</span>
            {#if recorded?.sha256}
                <span class="hash">{recorded.sha256.slice(0, 7)}</span>
            {/if}
            <span class="recorded-badge">Recorded</span>
            {#if recorded?.at}
                <span class="timestamp">{formatTimestamp(recorded.at)}</span>
            {/if}
            {#if onrevert}
                <button class="btn-delete" title="Delete recorded value" onclick={onrevert}>
                    <TrashIcon />
                </button>
            {/if}
        {:else if selectedFilename}
            <span class="filename">{selectedFilename}</span>
            <button class="btn-record" onclick={confirm} disabled={disabled}>
                Attach
            </button>
            <button class="btn-clear" onclick={clear} disabled={disabled}>
                Clear
            </button>
        {:else}
            <button class="btn-choose" onclick={pickFile} disabled={disabled}>
                Choose File
            </button>
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

    .input-row {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .filename {
        flex: 1;
        font-size: 13px;
        color: #333;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .btn-choose,
    .btn-record,
    .btn-clear {
        padding: 6px 12px;
        border: none;
        border-radius: 4px;
        font: inherit;
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;
        white-space: nowrap;
    }

    .btn-choose,
    .btn-record {
        background: #1a1a2e;
        color: #fff;
    }

    .btn-choose:hover:not(:disabled),
    .btn-record:hover:not(:disabled) {
        background: #16213e;
    }

    .btn-clear {
        background: #fff;
        color: #666;
        border: 1px solid #ccc;
    }

    .btn-clear:hover:not(:disabled) {
        background: #f5f5f5;
    }

    .btn-choose:disabled,
    .btn-record:disabled,
    .btn-clear:disabled {
        opacity: 0.4;
        cursor: not-allowed;
    }

    .hash {
        font-size: 11px;
        font-family: monospace;
        color: #888;
        white-space: nowrap;
    }

    .recorded-badge {
        font-size: 11px;
        font-weight: 600;
        color: #2e7d32;
        white-space: nowrap;
    }
</style>
