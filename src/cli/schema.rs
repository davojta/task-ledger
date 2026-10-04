use schemars::JsonSchema;
use serde::Serialize;

use super::output::{CheckEnvelope, ErrorEnvelope, ListEnvelope, TaskEnvelope};
use crate::core::error::{LedgerError, Result};

#[allow(dead_code, clippy::large_enum_variant)]
#[derive(JsonSchema, Serialize)]
#[serde(untagged)]
enum Output {
    List(ListEnvelope),
    Task(TaskEnvelope),
    Check(CheckEnvelope),
    Error(ErrorEnvelope),
}

pub fn run() -> Result<u8> {
    let schema = schemars::schema_for!(Output);
    let text = serde_json::to_string_pretty(&schema)
        .map_err(|e| LedgerError::Internal(format!("failed to serialize schema: {e}")))?;
    println!("{text}");
    Ok(0)
}
