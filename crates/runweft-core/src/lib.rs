//! Future authority boundary. Durable profile identity exists, but this scaffold
//! has no executor or network client.
pub mod ledger;
pub mod ownership;

pub fn status() -> runweft_protocol::RuntimeStatus {
    runweft_protocol::scaffold_status()
}
