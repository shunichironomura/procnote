import { invoke } from '@tauri-apps/api/core';
import type {
	TemplateSummary,
	ProcedureTemplate,
	ExecutionSummary,
	ExecutionAction
} from '$lib/types';

export async function listTemplates(): Promise<TemplateSummary[]> {
	return invoke('list_templates');
}

export async function loadTemplate(path: string): Promise<ProcedureTemplate> {
	return invoke('load_template', { path });
}

export async function startExecution(
	templatePath: string,
	operator: string
): Promise<ExecutionSummary> {
	return invoke('start_execution', { templatePath, operator });
}

export async function recordAction(
	executionId: string,
	action: ExecutionAction
): Promise<ExecutionSummary> {
	return invoke('record_action', { executionId, action });
}

export async function getExecutionState(executionId: string): Promise<ExecutionSummary> {
	return invoke('get_execution_state', { executionId });
}

export async function listExecutions(): Promise<ExecutionSummary[]> {
	return invoke('list_executions');
}
