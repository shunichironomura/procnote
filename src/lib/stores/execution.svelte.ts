import type { ExecutionSummary, ExecutionAction } from "$lib/types";
import * as api from "$lib/api/commands";

class ExecutionStore {
  summary: ExecutionSummary | null = $state(null);
  loading = $state(false);
  error: string | null = $state(null);

  get isActive() {
    return this.summary?.status === "active";
  }

  get isFinished() {
    const s = this.summary?.status;
    return s === "pass" || s === "fail" || s === "aborted";
  }

  async start(templatePath: string, operator: string) {
    this.loading = true;
    this.error = null;
    try {
      this.summary = await api.startExecution(templatePath, operator);
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loading = false;
    }
  }

  async load(executionId: string) {
    this.loading = true;
    this.error = null;
    try {
      this.summary = await api.getExecutionState(executionId);
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loading = false;
    }
  }

  async act(action: ExecutionAction) {
    if (!this.summary) return;
    this.error = null;
    try {
      this.summary = await api.recordAction(this.summary.execution_id, action);
    } catch (e) {
      this.error = String(e);
    }
  }

  reset() {
    this.summary = null;
    this.loading = false;
    this.error = null;
  }
}

export const executionStore = new ExecutionStore();
