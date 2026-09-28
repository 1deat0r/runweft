//! Future authority boundary. This scaffold has no executor, database or network client.
pub fn status() -> runweft_protocol::RuntimeStatus {
    runweft_protocol::scaffold_status()
}
