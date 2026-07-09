use anyhow::Result;
use dhara_tool_kernel::{has_committed_progress_plan, ProgressSession};

/// Runs one workflow step with progress when interactive or when a parent plan is active.
pub fn run_workflow_step(
    id: &'static str,
    label: &'static str,
    detail: &str,
    operation: impl FnOnce() -> Result<()>,
) -> Result<()> {
    if has_committed_progress_plan() {
        let session = ProgressSession;
        session.tick(id, 0, detail);
        let result = operation();
        if result.is_ok() {
            session.finish_step(id, label);
        }
        return result;
    }

    let session = ProgressSession;
    session.analyzing(format!("Preparing {label}…"));
    session.plan(id, label, 1);
    session.set_total(id, 1);
    session.commit();
    session.tick(id, 0, detail);
    let result = operation();
    if result.is_ok() {
        session.finish_step(id, label);
    }
    result
}

/// Begins a multi-step workflow unless a plan is already committed.
pub fn begin_workflow(analyzing: &str) -> Option<ProgressSession> {
    if has_committed_progress_plan() {
        return None;
    }
    let session = ProgressSession;
    session.analyzing(analyzing);
    Some(session)
}

pub fn plan_unit_step(session: &ProgressSession, id: &'static str, label: &'static str) {
    session.plan(id, label, 1);
    session.set_total(id, 1);
}

pub fn run_planned_step(
    id: &'static str,
    label: &'static str,
    detail: &str,
    operation: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let session = ProgressSession;
    session.tick(id, 0, detail);
    let result = operation();
    if result.is_ok() {
        session.finish_step(id, label);
    }
    result
}
