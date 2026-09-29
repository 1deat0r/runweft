//! SQLite-backed durable state. Runtime orchestration and external effects are not
//! implemented here; this module establishes profile identity and owner fencing.

use crate::ownership::{OwnershipError, ProfileOwnership};
use sha2::{Digest, Sha256};
use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

const APPLICATION_ID: i64 = 0x5257_4654; // "RWFT"
const SCHEMA_VERSION: i64 = 1;
const WRITER_QUEUE_CAPACITY: usize = 32;
const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(250);
const PROFILE_MARKER_SLOT_SIZE: usize = 512;
const PROFILE_MARKER_FILE_SIZE: usize = PROFILE_MARKER_SLOT_SIZE * 2;

const REQUIRED_TABLE_COLUMNS: &[(&str, &[&str])] = &[
    (
        "profile_state",
        &[
            "singleton",
            "profile_id",
            "profile_incarnation",
            "coordinator_generation",
            "quarantined",
            "restore_cutoff",
        ],
    ),
    (
        "command_results",
        &[
            "profile_id",
            "project_id",
            "profile_incarnation",
            "command_id",
            "intent_projection_json",
            "response_json",
            "event_cursor",
        ],
    ),
    (
        "events",
        &[
            "profile_id",
            "project_id",
            "profile_incarnation",
            "run_id",
            "sequence",
            "sequence_sort_key",
            "event_id",
            "coordinator_generation",
            "command_id",
            "event_json",
        ],
    ),
    (
        "projections",
        &[
            "profile_id",
            "project_id",
            "profile_incarnation",
            "entity_kind",
            "entity_id",
            "revision",
            "last_event_id",
            "body_json",
        ],
    ),
    (
        "budget_reservations",
        &[
            "profile_id",
            "project_id",
            "profile_incarnation",
            "run_id",
            "reservation_id",
            "command_id",
            "amount_micro_usd",
            "state",
        ],
    ),
    (
        "outbox",
        &[
            "dispatch_id",
            "profile_id",
            "project_id",
            "profile_incarnation",
            "command_id",
            "run_id",
            "effect_class",
            "state",
            "intent_json",
        ],
    ),
];

const REQUIRED_COLUMN_TYPES: &[(&str, &[&str])] = &[
    (
        "profile_state",
        &["INTEGER", "TEXT", "TEXT", "TEXT", "INTEGER", "TEXT"],
    ),
    (
        "command_results",
        &["TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT"],
    ),
    (
        "events",
        &[
            "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "BLOB", "TEXT", "TEXT", "TEXT", "TEXT",
        ],
    ),
    (
        "projections",
        &[
            "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT",
        ],
    ),
    (
        "budget_reservations",
        &[
            "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT",
        ],
    ),
    (
        "outbox",
        &[
            "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT",
        ],
    ),
];

const REQUIRED_PRIMARY_KEYS: &[(&str, &[&str])] = &[
    ("profile_state", &["singleton"]),
    (
        "command_results",
        &[
            "profile_id",
            "project_id",
            "profile_incarnation",
            "command_id",
        ],
    ),
    (
        "events",
        &[
            "profile_id",
            "project_id",
            "profile_incarnation",
            "run_id",
            "sequence_sort_key",
        ],
    ),
    (
        "projections",
        &[
            "profile_id",
            "project_id",
            "profile_incarnation",
            "entity_kind",
            "entity_id",
        ],
    ),
    (
        "budget_reservations",
        &[
            "profile_id",
            "project_id",
            "profile_incarnation",
            "run_id",
            "reservation_id",
        ],
    ),
    ("outbox", &["dispatch_id"]),
];

type ForeignKeyShape = (&'static str, &'static str, &'static str);

const REQUIRED_FOREIGN_KEYS: &[(&str, &[ForeignKeyShape])] = &[
    (
        "events",
        &[
            ("command_results", "profile_id", "profile_id"),
            ("command_results", "project_id", "project_id"),
            (
                "command_results",
                "profile_incarnation",
                "profile_incarnation",
            ),
            ("command_results", "command_id", "command_id"),
        ],
    ),
    ("projections", &[("events", "last_event_id", "event_id")]),
    (
        "budget_reservations",
        &[
            ("command_results", "profile_id", "profile_id"),
            ("command_results", "project_id", "project_id"),
            (
                "command_results",
                "profile_incarnation",
                "profile_incarnation",
            ),
            ("command_results", "command_id", "command_id"),
        ],
    ),
    (
        "outbox",
        &[
            ("command_results", "profile_id", "profile_id"),
            ("command_results", "project_id", "project_id"),
            (
                "command_results",
                "profile_incarnation",
                "profile_incarnation",
            ),
            ("command_results", "command_id", "command_id"),
        ],
    ),
];

const REQUIRED_CHECKS: &[(&str, &[&str])] = &[
    (
        "profile_state",
        &["check(singleton=1)", "check(quarantinedin(0,1))"],
    ),
    (
        "command_results",
        &[
            "check(json_valid(intent_projection_json))",
            "check(json_valid(response_json))",
        ],
    ),
    (
        "events",
        &[
            "check(length(sequence_sort_key)=8)",
            "check(json_valid(event_json))",
        ],
    ),
    ("projections", &["check(json_valid(body_json))"]),
    (
        "budget_reservations",
        &["check(statein('held','committed','released'))"],
    ),
    (
        "outbox",
        &[
            "check(effect_classin('read_only','remote_idempotent','reconcilable','opaque'))",
            "check(statein('pending','leased','completed','unknown_effect','cancelled'))",
            "check(json_valid(intent_json))",
        ],
    ),
];

const SCHEMA_V1: &str = r#"
CREATE TABLE profile_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    profile_id TEXT NOT NULL,
    profile_incarnation TEXT NOT NULL,
    coordinator_generation TEXT NOT NULL,
    quarantined INTEGER NOT NULL CHECK (quarantined IN (0, 1)),
    restore_cutoff TEXT
) STRICT;

CREATE TABLE command_results (
    profile_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    profile_incarnation TEXT NOT NULL,
    command_id TEXT NOT NULL,
    intent_projection_json TEXT NOT NULL CHECK (json_valid(intent_projection_json)),
    response_json TEXT NOT NULL CHECK (json_valid(response_json)),
    event_cursor TEXT NOT NULL,
    PRIMARY KEY (profile_id, project_id, profile_incarnation, command_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE events (
    profile_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    profile_incarnation TEXT NOT NULL,
    run_id TEXT NOT NULL,
    sequence TEXT NOT NULL,
    sequence_sort_key BLOB NOT NULL CHECK (length(sequence_sort_key) = 8),
    event_id TEXT NOT NULL UNIQUE,
    coordinator_generation TEXT NOT NULL,
    command_id TEXT NOT NULL,
    event_json TEXT NOT NULL CHECK (json_valid(event_json)),
    PRIMARY KEY (
        profile_id, project_id, profile_incarnation, run_id, sequence_sort_key
    ),
    FOREIGN KEY (profile_id, project_id, profile_incarnation, command_id)
        REFERENCES command_results (profile_id, project_id, profile_incarnation, command_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE projections (
    profile_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    profile_incarnation TEXT NOT NULL,
    entity_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    revision TEXT NOT NULL,
    last_event_id TEXT NOT NULL REFERENCES events (event_id),
    body_json TEXT NOT NULL CHECK (json_valid(body_json)),
    PRIMARY KEY (
        profile_id, project_id, profile_incarnation, entity_kind, entity_id
    )
) STRICT, WITHOUT ROWID;

CREATE TABLE budget_reservations (
    profile_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    profile_incarnation TEXT NOT NULL,
    run_id TEXT NOT NULL,
    reservation_id TEXT NOT NULL,
    command_id TEXT NOT NULL,
    amount_micro_usd TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('held', 'committed', 'released')),
    PRIMARY KEY (
        profile_id, project_id, profile_incarnation, run_id, reservation_id
    ),
    FOREIGN KEY (profile_id, project_id, profile_incarnation, command_id)
        REFERENCES command_results (profile_id, project_id, profile_incarnation, command_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE outbox (
    dispatch_id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    profile_incarnation TEXT NOT NULL,
    command_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    effect_class TEXT NOT NULL CHECK (
        effect_class IN ('read_only', 'remote_idempotent', 'reconcilable', 'opaque')
    ),
    state TEXT NOT NULL CHECK (
        state IN ('pending', 'leased', 'completed', 'unknown_effect', 'cancelled')
    ),
    intent_json TEXT NOT NULL CHECK (json_valid(intent_json)),
    FOREIGN KEY (profile_id, project_id, profile_incarnation, command_id)
        REFERENCES command_results (profile_id, project_id, profile_incarnation, command_id)
) STRICT;
"#;

/// The durable identity assigned to one profile incarnation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileIdentity {
    profile_id: String,
    profile_incarnation: String,
}

impl ProfileIdentity {
    pub fn new(
        profile_id: impl Into<String>,
        profile_incarnation: impl Into<String>,
    ) -> Result<Self, LedgerOpenError> {
        let profile_id = profile_id.into();
        let profile_incarnation = profile_incarnation.into();
        if !valid_identifier(&profile_id) || !valid_identifier(&profile_incarnation) {
            return Err(LedgerOpenError::InvalidProfileIdentity);
        }
        Ok(Self {
            profile_id,
            profile_incarnation,
        })
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn profile_incarnation(&self) -> &str {
        &self.profile_incarnation
    }
}

fn valid_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    value.len() <= 128
        && first.is_ascii_alphanumeric()
        && bytes
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileMarkerState {
    Initializing,
    Active,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProfileMarker {
    profile_id: String,
    profile_incarnation: String,
    generation: u64,
    sequence: u64,
    slot: usize,
    state: ProfileMarkerState,
}

#[cfg(target_os = "linux")]
struct ParsedProfileMarker {
    marker: Option<ProfileMarker>,
    has_damaged_slot: bool,
}

impl ProfileMarker {
    fn initializing(identity: ProfileIdentity) -> Self {
        Self {
            profile_id: identity.profile_id,
            profile_incarnation: identity.profile_incarnation,
            generation: 0,
            sequence: 1,
            slot: 0,
            state: ProfileMarkerState::Initializing,
        }
    }

    fn next_sequence(&self) -> Result<u64, LedgerOpenError> {
        self.sequence
            .checked_add(1)
            .ok_or(LedgerOpenError::ProfileMarkerSequenceExhausted)
    }

    fn active(&self, generation: u64) -> Result<Self, LedgerOpenError> {
        if generation <= self.generation {
            return Err(LedgerOpenError::IncompatibleDatabase);
        }
        Ok(Self {
            profile_id: self.profile_id.clone(),
            profile_incarnation: self.profile_incarnation.clone(),
            generation,
            sequence: self.next_sequence()?,
            slot: 1 - self.slot,
            state: ProfileMarkerState::Active,
        })
    }
}

#[cfg(target_os = "linux")]
fn read_profile_marker(file: &std::fs::File) -> Result<Vec<u8>, LedgerOpenError> {
    use std::io::{Read, Seek, SeekFrom};

    if file.metadata()?.len() > PROFILE_MARKER_FILE_SIZE as u64 {
        return Err(LedgerOpenError::CorruptProfileMarker);
    }
    let mut file = file;
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn parse_profile_marker(bytes: &[u8]) -> Result<ParsedProfileMarker, ()> {
    if bytes.len() > PROFILE_MARKER_FILE_SIZE {
        return Err(());
    }
    let mut records = Vec::new();
    let mut has_partial_or_corrupt_slot = false;
    for slot in 0..2 {
        let start = slot * PROFILE_MARKER_SLOT_SIZE;
        if start >= bytes.len() {
            continue;
        }
        let end = start + PROFILE_MARKER_SLOT_SIZE;
        let Some(record) = bytes.get(start..end) else {
            if bytes[start..].iter().any(|byte| *byte != 0) {
                has_partial_or_corrupt_slot = true;
            }
            continue;
        };
        if record.iter().all(|byte| *byte == 0) {
            continue;
        }
        match parse_profile_marker_slot(record, slot) {
            Some(marker) => records.push(marker),
            None => has_partial_or_corrupt_slot = true,
        }
    }

    records.sort_by_key(|marker| marker.sequence);
    match records.as_slice() {
        [] if has_partial_or_corrupt_slot => Err(()),
        [] => Ok(ParsedProfileMarker {
            marker: None,
            has_damaged_slot: false,
        }),
        [latest]
            if latest.state == ProfileMarkerState::Initializing && !has_partial_or_corrupt_slot =>
        {
            Ok(ParsedProfileMarker {
                marker: Some(latest.clone()),
                has_damaged_slot: false,
            })
        }
        [latest] if latest.state == ProfileMarkerState::Active => Ok(ParsedProfileMarker {
            marker: Some(latest.clone()),
            // A single active slot has no independent previous-state record. It
            // can recover only if SQLite proves progress beyond this generation.
            has_damaged_slot: true,
        }),
        [latest]
            if matches!(
                latest.state,
                ProfileMarkerState::Initializing | ProfileMarkerState::Active
            ) && has_partial_or_corrupt_slot =>
        {
            Ok(ParsedProfileMarker {
                marker: Some(latest.clone()),
                has_damaged_slot: true,
            })
        }
        [older, latest]
            if older.sequence.checked_add(1) == Some(latest.sequence)
                && latest.profile_id == older.profile_id
                && latest.profile_incarnation == older.profile_incarnation
                && latest.generation > older.generation
                && latest.state == ProfileMarkerState::Active
                && !has_partial_or_corrupt_slot =>
        {
            Ok(ParsedProfileMarker {
                marker: Some(latest.clone()),
                has_damaged_slot: false,
            })
        }
        _ => Err(()),
    }
}

#[cfg(target_os = "linux")]
fn parse_profile_marker_slot(bytes: &[u8], slot: usize) -> Option<ProfileMarker> {
    let text_len = bytes.iter().position(|byte| *byte == 0)?;
    if bytes[text_len..].iter().any(|byte| *byte != 0) {
        return None;
    }
    let text = std::str::from_utf8(&bytes[..text_len]).ok()?;
    let (payload, checksum) = text.rsplit_once('|')?;
    let expected_checksum = format!("{:x}", Sha256::digest(payload.as_bytes()));
    if checksum != expected_checksum {
        return None;
    }
    let fields = payload.split('|').collect::<Vec<_>>();
    if fields.len() != 6 || fields[0] != "RWFT_PROFILE_V1" {
        return None;
    }
    let sequence = parse_canonical_u64(fields[1])?;
    if sequence == 0 {
        return None;
    }
    let state = match fields[2] {
        "initializing" => ProfileMarkerState::Initializing,
        "active" => ProfileMarkerState::Active,
        _ => return None,
    };
    if ProfileIdentity::new(fields[3], fields[4]).is_err() {
        return None;
    }
    let generation = parse_canonical_u64(fields[5])?;
    match state {
        ProfileMarkerState::Initializing if sequence != 1 || generation != 0 => return None,
        ProfileMarkerState::Active if sequence < 2 || generation == 0 => return None,
        _ => {}
    }
    Some(ProfileMarker {
        profile_id: fields[3].to_owned(),
        profile_incarnation: fields[4].to_owned(),
        generation,
        sequence,
        slot,
        state,
    })
}

#[cfg(target_os = "linux")]
fn encode_profile_marker(marker: &ProfileMarker) -> Result<[u8; PROFILE_MARKER_SLOT_SIZE], ()> {
    let state = match marker.state {
        ProfileMarkerState::Initializing => "initializing",
        ProfileMarkerState::Active => "active",
    };
    let payload = format!(
        "RWFT_PROFILE_V1|{}|{}|{}|{}|{}",
        marker.sequence, state, marker.profile_id, marker.profile_incarnation, marker.generation
    );
    let record = format!("{payload}|{:x}", Sha256::digest(payload.as_bytes()));
    if record.len() >= PROFILE_MARKER_SLOT_SIZE {
        return Err(());
    }
    let mut bytes = [0; PROFILE_MARKER_SLOT_SIZE];
    bytes[..record.len()].copy_from_slice(record.as_bytes());
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn write_initial_profile_marker(
    ownership: &ProfileOwnership,
    marker: &ProfileMarker,
) -> Result<(), LedgerOpenError> {
    use std::os::unix::fs::FileExt;

    let record =
        encode_profile_marker(marker).map_err(|()| LedgerOpenError::CorruptProfileMarker)?;
    let file = ownership.marker_file();
    file.set_len(0)?;
    file.write_all_at(&record, 0)?;
    file.sync_all()?;
    ownership.sync_profile_directory()?;
    verify_profile_marker(ownership, marker)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn write_profile_marker(
    ownership: &ProfileOwnership,
    marker: &ProfileMarker,
) -> Result<(), LedgerOpenError> {
    use std::os::unix::fs::FileExt;

    let record =
        encode_profile_marker(marker).map_err(|()| LedgerOpenError::CorruptProfileMarker)?;
    let offset = (marker.slot * PROFILE_MARKER_SLOT_SIZE) as u64;
    let file = ownership.marker_file();
    file.write_all_at(&[0; PROFILE_MARKER_SLOT_SIZE], offset)?;
    file.write_all_at(&record, offset)?;
    file.sync_all()?;
    ownership.sync_profile_directory()?;
    verify_profile_marker(ownership, marker)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_profile_marker(
    ownership: &ProfileOwnership,
    expected: &ProfileMarker,
) -> Result<(), LedgerOpenError> {
    let bytes = read_profile_marker(ownership.marker_file())?;
    let actual =
        parse_profile_marker(&bytes).map_err(|()| LedgerOpenError::CorruptProfileMarker)?;
    if actual.marker.as_ref() != Some(expected) {
        return Err(LedgerOpenError::CorruptProfileMarker);
    }
    Ok(())
}

/// An opened SQLite profile. The OS ownership lock remains held through writer
/// shutdown and is released only after the writer thread has stopped.
pub struct Ledger {
    identity: ProfileIdentity,
    coordinator_generation: u64,
    writer_sender: SyncSender<WriterCommand>,
    writer_thread: Option<JoinHandle<()>>,
    _ownership: ProfileOwnership,
}

enum WriterCommand {
    Shutdown,
}

impl Ledger {
    /// Open an initialized profile and durably advance its owner generation before
    /// returning it to a coordinator. Missing profiles must be created explicitly.
    pub fn open(profile_dir: &Path, identity: ProfileIdentity) -> Result<Self, LedgerOpenError> {
        Self::open_with_creation(profile_dir, identity, false)
    }

    /// Explicitly provision a private profile or resume intact, incomplete
    /// provisioning. Coordinator startup must use `open` and must not route missing
    /// or corrupt state through this operation.
    pub fn create(profile_dir: &Path, identity: ProfileIdentity) -> Result<Self, LedgerOpenError> {
        Self::open_with_creation(profile_dir, identity, true)
    }

    fn open_with_creation(
        profile_dir: &Path,
        identity: ProfileIdentity,
        create_if_missing: bool,
    ) -> Result<Self, LedgerOpenError> {
        #[cfg(target_os = "linux")]
        {
            Self::open_linux(profile_dir, identity, create_if_missing)
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (profile_dir, identity, create_if_missing);
            Err(LedgerOpenError::UnsupportedPlatform)
        }
    }

    #[cfg(target_os = "linux")]
    fn open_linux(
        profile_dir: &Path,
        identity: ProfileIdentity,
        create_if_missing: bool,
    ) -> Result<Self, LedgerOpenError> {
        let ownership = ProfileOwnership::acquire(profile_dir)?;
        let has_database = ownership.prepare_ledger_file(false)?;
        let marker_bytes = read_profile_marker(ownership.marker_file())?;
        let ParsedProfileMarker {
            marker: parsed_marker,
            has_damaged_slot: marker_has_damaged_slot,
        } = parse_profile_marker(&marker_bytes)
            .map_err(|()| LedgerOpenError::CorruptProfileMarker)?;
        let marker = match parsed_marker {
            Some(marker) => marker,
            None if !has_database && create_if_missing => {
                let marker = ProfileMarker::initializing(identity.clone());
                write_initial_profile_marker(&ownership, &marker)?;
                marker
            }
            None if !has_database => return Err(LedgerOpenError::ProfileNotInitialized),
            None => return Err(LedgerOpenError::MissingProfileMarker),
        };
        if create_if_missing && marker.state == ProfileMarkerState::Active {
            return Err(LedgerOpenError::ProfileAlreadyInitialized);
        }
        if marker.profile_id != identity.profile_id
            || marker.profile_incarnation != identity.profile_incarnation
        {
            return Err(LedgerOpenError::ProfileIdentityMismatch);
        }
        marker.next_sequence()?;
        let may_initialize_database =
            create_if_missing && marker.state == ProfileMarkerState::Initializing;
        if !has_database && !may_initialize_database {
            return Err(if marker.state == ProfileMarkerState::Initializing {
                LedgerOpenError::ProfileNotInitialized
            } else {
                LedgerOpenError::MissingLedger
            });
        }
        if !has_database && marker_has_damaged_slot {
            return Err(LedgerOpenError::CorruptProfileMarker);
        }
        ownership.prepare_ledger_file(may_initialize_database)?;
        let database_path = ownership.ledger_path();
        let (connection, coordinator_generation) = initialize_database(
            &database_path,
            &identity,
            marker.generation,
            may_initialize_database,
            marker_has_damaged_slot,
        )?;
        let active_marker = marker.active(coordinator_generation)?;
        write_profile_marker(&ownership, &active_marker)?;
        let (writer_sender, writer_receiver) = mpsc::sync_channel(WRITER_QUEUE_CAPACITY);
        let writer_thread = thread::Builder::new()
            .name("runweft-ledger-writer".to_string())
            .spawn(move || writer_loop(connection, writer_receiver))
            .map_err(LedgerOpenError::WriterStart)?;

        Ok(Self {
            identity,
            coordinator_generation,
            writer_sender,
            writer_thread: Some(writer_thread),
            _ownership: ownership,
        })
    }

    pub fn identity(&self) -> &ProfileIdentity {
        &self.identity
    }

    pub fn coordinator_generation(&self) -> u64 {
        self.coordinator_generation
    }
}

impl Drop for Ledger {
    fn drop(&mut self) {
        let _ = self.writer_sender.send(WriterCommand::Shutdown);
        if let Some(writer_thread) = self.writer_thread.take() {
            let _ = writer_thread.join();
        }
    }
}

fn writer_loop(connection: rusqlite::Connection, receiver: Receiver<WriterCommand>) {
    let _connection = connection;
    let _ = receiver.recv();
}

#[cfg(target_os = "linux")]
fn initialize_database(
    path: &Path,
    identity: &ProfileIdentity,
    marker_generation: u64,
    allow_new_database: bool,
    marker_has_damaged_slot: bool,
) -> Result<(rusqlite::Connection, u64), LedgerOpenError> {
    use rusqlite::{Connection, TransactionBehavior};

    let mut connection = Connection::open(path)?;
    connection.busy_timeout(BUSY_TIMEOUT)?;

    let application_id: i64 =
        connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let user_version: i64 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if application_id == 0 {
        if marker_has_damaged_slot {
            return Err(LedgerOpenError::CorruptProfileMarker);
        }
        let user_schema_object_count: i64 =
            connection.query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))?;
        if !allow_new_database || user_version != 0 || user_schema_object_count != 0 {
            return Err(LedgerOpenError::IncompatibleDatabase);
        }
    } else if application_id != APPLICATION_ID || user_version != SCHEMA_VERSION {
        return Err(LedgerOpenError::IncompatibleDatabase);
    } else {
        validate_required_schema(&connection)?;
    }

    // Reject a different or malformed profile before any persistent journal-mode
    // change. Re-read this state inside the write transaction below before the
    // generation update.
    if application_id != 0 {
        let generation = validate_profile_state(
            &connection,
            identity,
            marker_generation,
            marker_has_damaged_slot,
        )?;
        generation
            .checked_add(1)
            .ok_or(LedgerOpenError::GenerationExhausted)?;
    }

    // Validate the database identity before changing its journal mode. This avoids
    // mutating an unrelated SQLite file placed in the profile directory.
    let journal_mode: String =
        connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    if journal_mode != "wal" {
        return Err(LedgerOpenError::UnsupportedJournalMode(journal_mode));
    }
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "trusted_schema", "OFF")?;

    if application_id == 0 {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Exclusive)?;
        transaction.execute_batch(SCHEMA_V1)?;
        transaction.execute(
            "INSERT INTO profile_state \
             (singleton, profile_id, profile_incarnation, coordinator_generation, quarantined) \
             VALUES (1, ?1, ?2, '1', 0)",
            (&identity.profile_id, &identity.profile_incarnation),
        )?;
        transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
        transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        transaction.commit()?;
        return Ok((connection, 1));
    }

    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let generation = validate_profile_state(
        &transaction,
        identity,
        marker_generation,
        marker_has_damaged_slot,
    )?;
    let next_generation = generation
        .checked_add(1)
        .ok_or(LedgerOpenError::GenerationExhausted)?;
    let updated = transaction.execute(
        "UPDATE profile_state SET coordinator_generation = ?1 WHERE singleton = 1",
        [next_generation.to_string()],
    )?;
    if updated != 1 {
        return Err(LedgerOpenError::IncompatibleDatabase);
    }
    transaction.commit()?;
    Ok((connection, next_generation))
}

#[cfg(target_os = "linux")]
fn validate_profile_state(
    connection: &rusqlite::Connection,
    identity: &ProfileIdentity,
    marker_generation: u64,
    marker_has_damaged_slot: bool,
) -> Result<u64, LedgerOpenError> {
    use rusqlite::OptionalExtension;

    let stored = connection
        .query_row(
            "SELECT profile_id, profile_incarnation, coordinator_generation, quarantined, \
                    restore_cutoff \
             FROM profile_state WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or(LedgerOpenError::IncompatibleDatabase)?;
    if stored.0 != identity.profile_id || stored.1 != identity.profile_incarnation {
        return Err(LedgerOpenError::ProfileIdentityMismatch);
    }
    if stored.3 == 1 {
        return Err(LedgerOpenError::ProfileQuarantined);
    }
    if stored.3 != 0 || stored.4.is_some() {
        return Err(LedgerOpenError::IncompatibleDatabase);
    }
    let generation =
        parse_canonical_generation(&stored.2).ok_or(LedgerOpenError::IncompatibleDatabase)?;
    if generation < marker_generation {
        return Err(LedgerOpenError::GenerationRollback {
            database: generation,
            marker: marker_generation,
        });
    }
    if marker_has_damaged_slot && generation <= marker_generation {
        return Err(LedgerOpenError::CorruptProfileMarker);
    }
    Ok(generation)
}

#[cfg(target_os = "linux")]
fn validate_required_schema(connection: &rusqlite::Connection) -> Result<(), LedgerOpenError> {
    // Build the expected v1 schema with this same bundled SQLite runtime and
    // compare its full catalog. This checks constraint grouping and expressions
    // structurally; textual substring probes alone can be spoofed by comments or
    // string literals in a modified schema.
    let canonical = rusqlite::Connection::open_in_memory()?;
    canonical.execute_batch(SCHEMA_V1)?;
    if schema_signature(connection)? != schema_signature(&canonical)? {
        return Err(LedgerOpenError::IncompatibleDatabase);
    }

    for (table, expected_columns) in REQUIRED_TABLE_COLUMNS {
        let sql = format!("PRAGMA table_info({table})");
        let mut statement = connection.prepare(&sql)?;
        let actual_columns = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(5)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let expected_types = REQUIRED_COLUMN_TYPES
            .iter()
            .find(|(expected_table, _)| expected_table == table)
            .map(|(_, types)| *types)
            .ok_or(LedgerOpenError::IncompatibleDatabase)?;
        if actual_columns.len() != expected_columns.len()
            || actual_columns.len() != expected_types.len()
            || expected_columns
                .iter()
                .zip(actual_columns.iter())
                .zip(expected_types.iter())
                .any(|((expected_name, actual), expected_type)| {
                    *expected_name != actual.0 || *expected_type != actual.1
                })
        {
            return Err(LedgerOpenError::IncompatibleDatabase);
        }
        if expected_columns
            .iter()
            .zip(actual_columns.iter())
            .any(|(name, (_, _, not_null, _))| {
                *not_null == 0 && !matches!(*name, "singleton" | "restore_cutoff")
            })
        {
            return Err(LedgerOpenError::IncompatibleDatabase);
        }

        let actual_primary_key = actual_columns
            .iter()
            .filter(|(_, _, _, ordinal)| *ordinal > 0)
            .collect::<Vec<_>>();
        let mut actual_primary_key = actual_primary_key;
        actual_primary_key.sort_by_key(|(_, _, _, ordinal)| *ordinal);
        let expected_primary_key = REQUIRED_PRIMARY_KEYS
            .iter()
            .find(|(expected_table, _)| expected_table == table)
            .map(|(_, columns)| *columns)
            .ok_or(LedgerOpenError::IncompatibleDatabase)?;
        if actual_primary_key.len() != expected_primary_key.len()
            || expected_primary_key
                .iter()
                .zip(actual_primary_key.iter())
                .any(|(expected, actual)| *expected != actual.0)
        {
            return Err(LedgerOpenError::IncompatibleDatabase);
        }

        let sql: String = connection.query_row(
            "SELECT sql FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )?;
        let compact_sql = sql
            .chars()
            .filter(|character| !character.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect::<String>();
        let required_checks = REQUIRED_CHECKS
            .iter()
            .find(|(expected_table, _)| expected_table == table)
            .map(|(_, checks)| *checks)
            .ok_or(LedgerOpenError::IncompatibleDatabase)?;
        if required_checks
            .iter()
            .any(|check| !compact_sql.contains(check))
        {
            return Err(LedgerOpenError::IncompatibleDatabase);
        }
    }

    validate_table_modes(connection)?;
    validate_required_foreign_keys(connection)?;
    validate_event_id_uniqueness(connection)?;

    let unexpected_schema_objects: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema \
         WHERE substr(name, 1, 7) COLLATE NOCASE != 'sqlite_' AND type != 'table'",
        [],
        |row| row.get(0),
    )?;
    let expected_table_count = REQUIRED_TABLE_COLUMNS.len() as i64;
    let actual_table_count: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema \
         WHERE substr(name, 1, 7) COLLATE NOCASE != 'sqlite_' AND type = 'table'",
        [],
        |row| row.get(0),
    )?;
    if unexpected_schema_objects != 0 || actual_table_count != expected_table_count {
        return Err(LedgerOpenError::IncompatibleDatabase);
    }

    // The profile row is checked separately; this query confirms the table is
    // exactly the expected singleton shape before any journal-mode mutation.
    let singleton_count = connection.query_row(
        "SELECT count(*) FROM profile_state WHERE singleton = 1",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    if singleton_count != 1 {
        return Err(LedgerOpenError::IncompatibleDatabase);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
type SqliteSchemaObject = (String, String, String, Option<String>);

#[cfg(target_os = "linux")]
fn schema_signature(
    connection: &rusqlite::Connection,
) -> Result<Vec<SqliteSchemaObject>, LedgerOpenError> {
    let mut statement = connection.prepare(
        "SELECT type, name, tbl_name, sql FROM sqlite_schema ORDER BY type, name, tbl_name",
    )?;
    let mut objects = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    objects.sort();
    Ok(objects)
}

#[cfg(target_os = "linux")]
fn validate_table_modes(connection: &rusqlite::Connection) -> Result<(), LedgerOpenError> {
    let mut statement = connection.prepare("PRAGMA table_list")?;
    let tables = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (table, _) in REQUIRED_TABLE_COLUMNS {
        let (_, kind, without_rowid, strict) = tables
            .iter()
            .find(|(name, _, _, _)| name == table)
            .ok_or(LedgerOpenError::IncompatibleDatabase)?;
        let expected_without_rowid = matches!(
            *table,
            "command_results" | "events" | "projections" | "budget_reservations"
        );
        if kind != "table" || *strict != 1 || (*without_rowid == 1) != expected_without_rowid {
            return Err(LedgerOpenError::IncompatibleDatabase);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_required_foreign_keys(
    connection: &rusqlite::Connection,
) -> Result<(), LedgerOpenError> {
    for (table, expected) in REQUIRED_FOREIGN_KEYS {
        let pragma = format!("PRAGMA foreign_key_list({table})");
        let mut statement = connection.prepare(&pragma)?;
        let actual = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut expected = expected
            .iter()
            .map(|(target, from, to)| {
                (
                    target.to_string(),
                    from.to_string(),
                    to.to_string(),
                    "NO ACTION".to_string(),
                    "NO ACTION".to_string(),
                    "NONE".to_string(),
                )
            })
            .collect::<Vec<_>>();
        let mut actual = actual;
        expected.sort();
        actual.sort();
        if actual != expected {
            return Err(LedgerOpenError::IncompatibleDatabase);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_event_id_uniqueness(connection: &rusqlite::Connection) -> Result<(), LedgerOpenError> {
    let mut statement = connection.prepare("PRAGMA index_list(events)")?;
    let indexes = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (index_name, unique, partial) in indexes {
        if unique != 1 || partial != 0 {
            continue;
        }
        let mut index_info = connection.prepare("SELECT name FROM pragma_index_info(?1)")?;
        let columns = index_info
            .query_map([index_name], |row| row.get::<_, Option<String>>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        if columns == [Some("event_id".to_string())] {
            return Ok(());
        }
    }
    Err(LedgerOpenError::IncompatibleDatabase)
}

#[cfg(target_os = "linux")]
fn parse_canonical_generation(value: &str) -> Option<u64> {
    let generation = value.parse::<u64>().ok()?;
    (generation > 0 && generation.to_string() == value).then_some(generation)
}

#[cfg(target_os = "linux")]
fn parse_canonical_u64(value: &str) -> Option<u64> {
    let number = value.parse::<u64>().ok()?;
    (number.to_string() == value).then_some(number)
}

#[derive(Debug)]
pub enum LedgerOpenError {
    Ownership(OwnershipError),
    Sqlite(rusqlite::Error),
    WriterStart(io::Error),
    InvalidProfileIdentity,
    IncompatibleDatabase,
    ProfileIdentityMismatch,
    GenerationExhausted,
    GenerationRollback { database: u64, marker: u64 },
    MissingLedger,
    MissingProfileMarker,
    ProfileNotInitialized,
    ProfileAlreadyInitialized,
    ProfileQuarantined,
    CorruptProfileMarker,
    ProfileMarkerSequenceExhausted,
    ProfileMarkerIo(io::Error),
    UnsupportedJournalMode(String),
    UnsupportedPlatform,
}

impl std::fmt::Display for LedgerOpenError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ownership(error) => write!(formatter, "profile ownership failed: {error}"),
            Self::Sqlite(error) => write!(formatter, "SQLite ledger failed: {error}"),
            Self::WriterStart(error) => write!(formatter, "ledger writer failed to start: {error}"),
            Self::ProfileMarkerIo(error) => write!(formatter, "profile marker I/O failed: {error}"),
            Self::InvalidProfileIdentity => {
                formatter.write_str("profile identity fields must be nonempty bounded identifiers")
            }
            Self::IncompatibleDatabase => {
                formatter.write_str("profile database identity or schema is incompatible")
            }
            Self::ProfileIdentityMismatch => {
                formatter.write_str("profile database belongs to a different incarnation")
            }
            Self::GenerationExhausted => formatter.write_str("coordinator generation is exhausted"),
            Self::GenerationRollback { database, marker } => write!(
                formatter,
                "database generation {database} is older than profile marker generation {marker}"
            ),
            Self::MissingLedger => formatter.write_str("initialized profile ledger is missing"),
            Self::MissingProfileMarker => {
                formatter.write_str("database exists without its durable profile marker")
            }
            Self::ProfileNotInitialized => {
                formatter.write_str("profile has not been explicitly initialized")
            }
            Self::ProfileAlreadyInitialized => {
                formatter.write_str("profile is already initialized")
            }
            Self::ProfileQuarantined => {
                formatter.write_str("profile is quarantined and cannot accept normal opens")
            }
            Self::CorruptProfileMarker => formatter.write_str("profile marker is corrupt"),
            Self::ProfileMarkerSequenceExhausted => {
                formatter.write_str("profile marker sequence is exhausted")
            }
            Self::UnsupportedJournalMode(mode) => {
                write!(
                    formatter,
                    "SQLite could not enable WAL mode (returned {mode})"
                )
            }
            Self::UnsupportedPlatform => {
                formatter.write_str("the durable profile ledger is supported on Linux only")
            }
        }
    }
}

impl std::error::Error for LedgerOpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ownership(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::WriterStart(error) => Some(error),
            Self::ProfileMarkerIo(error) => Some(error),
            Self::InvalidProfileIdentity
            | Self::IncompatibleDatabase
            | Self::ProfileIdentityMismatch
            | Self::GenerationExhausted
            | Self::GenerationRollback { .. }
            | Self::MissingLedger
            | Self::MissingProfileMarker
            | Self::ProfileNotInitialized
            | Self::ProfileAlreadyInitialized
            | Self::ProfileQuarantined
            | Self::CorruptProfileMarker
            | Self::ProfileMarkerSequenceExhausted
            | Self::UnsupportedJournalMode(_)
            | Self::UnsupportedPlatform => None,
        }
    }
}

impl From<OwnershipError> for LedgerOpenError {
    fn from(error: OwnershipError) -> Self {
        Self::Ownership(error)
    }
}

impl From<io::Error> for LedgerOpenError {
    fn from(error: io::Error) -> Self {
        Self::ProfileMarkerIo(error)
    }
}

impl From<rusqlite::Error> for LedgerOpenError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{
        APPLICATION_ID, Ledger, LedgerOpenError, PROFILE_MARKER_FILE_SIZE,
        PROFILE_MARKER_SLOT_SIZE, ProfileIdentity, ProfileMarker, ProfileMarkerState,
        SCHEMA_VERSION, encode_profile_marker, initialize_database, parse_profile_marker,
        write_initial_profile_marker,
    };
    use crate::ownership::{OwnershipError, ProfileOwnership};
    use rusqlite::Connection;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_PROFILE: AtomicUsize = AtomicUsize::new(0);

    struct TempProfile(PathBuf);

    impl TempProfile {
        fn new() -> Self {
            let sequence = NEXT_PROFILE.fetch_add(1, Ordering::Relaxed);
            let test_root = std::env::var_os("HOME")
                .map(PathBuf::from)
                .expect("Linux test environment has a home directory")
                .join(".runweft-test-profiles");
            fs::create_dir_all(&test_root).expect("create local test directory");
            fs::set_permissions(&test_root, fs::Permissions::from_mode(0o700))
                .expect("make test directory private");
            let path = test_root.join(format!("runweft-ledger-{}-{sequence}", std::process::id()));
            fs::create_dir(&path).expect("create temporary profile directory");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .expect("make temporary profile private");
            Self(path)
        }
    }

    impl Drop for TempProfile {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn identity(incarnation: &str) -> ProfileIdentity {
        ProfileIdentity::new("profile-test", incarnation).expect("valid profile identity")
    }

    #[test]
    fn marker_parser_fails_closed_if_the_new_profile_record_is_torn() {
        let initializing = ProfileMarker::initializing(identity("incarnation-test"));
        let active = initializing.active(1).expect("valid active marker");
        let mut bytes = vec![0; PROFILE_MARKER_FILE_SIZE];
        bytes[..PROFILE_MARKER_SLOT_SIZE]
            .copy_from_slice(&encode_profile_marker(&initializing).expect("encode bootstrap"));
        bytes[PROFILE_MARKER_SLOT_SIZE..]
            .copy_from_slice(&encode_profile_marker(&active).expect("encode active marker"));
        bytes[PROFILE_MARKER_SLOT_SIZE + 100] ^= 1;

        let parsed = parse_profile_marker(&bytes).expect("bootstrap marker is recoverable");
        assert_eq!(parsed.marker, Some(initializing));
        assert!(parsed.has_damaged_slot);
    }

    #[test]
    fn marker_parser_recovers_from_a_torn_update_using_the_previous_active_slot() {
        let initializing = ProfileMarker::initializing(identity("incarnation-test"));
        let active_one = initializing.active(1).expect("first active marker");
        let active_two = active_one.active(2).expect("second active marker");
        let mut bytes = vec![0; PROFILE_MARKER_FILE_SIZE];
        bytes[..PROFILE_MARKER_SLOT_SIZE]
            .copy_from_slice(&encode_profile_marker(&active_two).expect("encode new active"));
        bytes[PROFILE_MARKER_SLOT_SIZE..]
            .copy_from_slice(&encode_profile_marker(&active_one).expect("encode old active"));
        bytes[100] ^= 1;

        let parsed = parse_profile_marker(&bytes).expect("prior active slot remains recoverable");
        let marker = parsed.marker.expect("previous active slot is valid");
        assert_eq!(marker.generation, 1);
        assert_eq!(marker.state, ProfileMarkerState::Active);
        assert_eq!(marker.sequence, 2);
        assert!(parsed.has_damaged_slot);
    }

    #[test]
    fn pinned_profile_directory_survives_path_replacement() {
        let profile = TempProfile::new();
        let original_directory = profile.0.with_extension("pinned-original");
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        fs::rename(&profile.0, &original_directory).expect("rename opened directory");
        fs::create_dir(&profile.0).expect("replace original path");
        fs::set_permissions(&profile.0, fs::Permissions::from_mode(0o700))
            .expect("make replacement private");

        assert!(
            !owner
                .prepare_ledger_file(true)
                .expect("create database by pinned directory handle")
        );
        let (connection, generation) = initialize_database(
            &owner.ledger_path(),
            &identity("incarnation-test"),
            0,
            true,
            false,
        )
        .expect("initialize database in pinned directory");
        assert_eq!(generation, 1);
        drop(connection);
        assert!(original_directory.join("ledger.sqlite").exists());
        assert!(!profile.0.join("ledger.sqlite").exists());

        drop(owner);
        fs::remove_dir_all(original_directory).expect("remove pinned test directory");
    }

    #[test]
    fn initializes_wal_full_sync_schema_and_advances_generation_on_each_owner() {
        let profile = TempProfile::new();
        let first = Ledger::create(&profile.0, identity("incarnation-test"))
            .expect("first coordinator opens profile");
        assert_eq!(first.coordinator_generation(), 1);
        assert!(matches!(
            ProfileOwnership::acquire(&profile.0),
            Err(OwnershipError::AlreadyOwned)
        ));
        drop(first);

        let second = Ledger::open(&profile.0, identity("incarnation-test"))
            .expect("replacement coordinator opens profile");
        assert_eq!(second.coordinator_generation(), 2);
        assert_eq!(second.identity().profile_id(), "profile-test");

        let owner = ProfileOwnership::acquire(&profile.0);
        assert!(matches!(owner, Err(OwnershipError::AlreadyOwned)));
        drop(second);

        let owner = ProfileOwnership::acquire(&profile.0).expect("lock releases after shutdown");
        let path = owner.ledger_path();
        let (connection, generation) =
            initialize_database(&path, &identity("incarnation-test"), 2, false, false)
                .expect("reopen for probes");
        assert_eq!(generation, 3);
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let synchronous: i64 = connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .expect("read synchronous mode");
        let foreign_keys: i64 = connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .expect("read foreign key mode");
        let app_id: i64 = connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .expect("read application id");
        let user_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(journal, "wal");
        assert_eq!(synchronous, 2, "SQLite FULL is numeric value 2");
        assert_eq!(foreign_keys, 1);
        assert_eq!(app_id, APPLICATION_ID);
        assert_eq!(user_version, SCHEMA_VERSION);
        let foreign_key_failure = connection.execute(
            "INSERT INTO events (profile_id, project_id, profile_incarnation, run_id, \
             sequence, sequence_sort_key, event_id, coordinator_generation, command_id, event_json) \
             VALUES ('profile-test', 'project-test', 'incarnation-test', 'run-test', '1', \
             X'0000000000000000', 'event-test', '1', 'missing-command', '{}')",
            [],
        );
        assert!(matches!(
            foreign_key_failure,
            Err(rusqlite::Error::SqliteFailure(ref error, _))
                if error.code == rusqlite::ErrorCode::ConstraintViolation
        ));
    }

    #[test]
    fn resumes_interrupted_first_creation_without_reusing_a_generation() {
        let profile = TempProfile::new();
        let identity = identity("incarnation-test");
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        let initializing = ProfileMarker::initializing(identity.clone());
        write_initial_profile_marker(&owner, &initializing).expect("persist bootstrap marker");
        assert!(
            !owner
                .prepare_ledger_file(true)
                .expect("create interrupted database")
        );
        let (connection, generation) =
            initialize_database(&owner.ledger_path(), &identity, 0, true, false)
                .expect("commit initial schema before simulated crash");
        assert_eq!(generation, 1);
        drop(connection);
        drop(owner);

        let reopened = Ledger::open(&profile.0, identity).expect("resume first creation");
        assert_eq!(reopened.coordinator_generation(), 2);
    }

    #[test]
    fn normal_open_does_not_create_a_database_from_an_initializing_marker() {
        let profile = TempProfile::new();
        let profile_identity = identity("incarnation-test");
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        write_initial_profile_marker(
            &owner,
            &ProfileMarker::initializing(profile_identity.clone()),
        )
        .expect("persist start of explicit provisioning");
        drop(owner);

        assert!(matches!(
            Ledger::open(&profile.0, profile_identity.clone()),
            Err(LedgerOpenError::ProfileNotInitialized)
        ));
        assert!(
            !profile.0.join("ledger.sqlite").exists(),
            "normal open must not create a ledger from an in-progress marker"
        );

        let created =
            Ledger::create(&profile.0, profile_identity).expect("explicit create resumes setup");
        assert_eq!(created.coordinator_generation(), 1);
    }

    #[test]
    fn refuses_exhausted_marker_sequence_before_database_mutation() {
        use std::os::unix::fs::FileExt;

        let profile = TempProfile::new();
        let profile_identity = identity("incarnation-test");
        drop(Ledger::create(&profile.0, profile_identity.clone()).expect("create profile"));
        drop(Ledger::open(&profile.0, profile_identity.clone()).expect("advance to generation 2"));

        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        let older_marker = ProfileMarker {
            profile_id: profile_identity.profile_id().to_owned(),
            profile_incarnation: profile_identity.profile_incarnation().to_owned(),
            generation: 1,
            sequence: u64::MAX - 1,
            slot: 0,
            state: ProfileMarkerState::Active,
        };
        let exhausted_marker = ProfileMarker {
            profile_id: profile_identity.profile_id().to_owned(),
            profile_incarnation: profile_identity.profile_incarnation().to_owned(),
            generation: 2,
            sequence: u64::MAX,
            slot: 1,
            state: ProfileMarkerState::Active,
        };
        owner
            .marker_file()
            .write_at(
                &encode_profile_marker(&older_marker).expect("encode previous marker"),
                0,
            )
            .expect("write previous marker slot");
        owner
            .marker_file()
            .write_at(
                &encode_profile_marker(&exhausted_marker).expect("encode exhausted marker"),
                PROFILE_MARKER_SLOT_SIZE as u64,
            )
            .expect("write exhausted marker slot");
        owner.marker_file().sync_all().expect("sync marker slots");
        drop(owner);

        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("open database for assertions");
        assert_eq!(
            connection
                .query_row("PRAGMA journal_mode = DELETE", [], |row| row
                    .get::<_, String>(0))
                .expect("set delete journal mode"),
            "delete"
        );
        drop(connection);

        assert!(matches!(
            Ledger::open(&profile.0, profile_identity),
            Err(LedgerOpenError::ProfileMarkerSequenceExhausted)
        ));

        let connection = Connection::open(database).expect("reopen database after rejection");
        let generation: String = connection
            .query_row(
                "SELECT coordinator_generation FROM profile_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("read unchanged generation");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read unchanged journal mode");
        assert_eq!(generation, "2");
        assert_eq!(journal, "delete");
    }

    #[test]
    fn rejects_unexpected_schema_objects_before_bootstrap_mutation() {
        let profile = TempProfile::new();
        let profile_identity = identity("incarnation-test");
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        write_initial_profile_marker(
            &owner,
            &ProfileMarker::initializing(profile_identity.clone()),
        )
        .expect("persist start of explicit provisioning");
        assert!(
            !owner
                .prepare_ledger_file(true)
                .expect("create empty bootstrap database")
        );
        let connection = Connection::open(owner.ledger_path()).expect("open bootstrap database");
        connection
            .execute_batch("CREATE VIEW sqliteXshadow AS SELECT 1;")
            .expect("add unexpected view");
        drop(connection);
        drop(owner);

        assert!(matches!(
            Ledger::create(&profile.0, profile_identity),
            Err(LedgerOpenError::IncompatibleDatabase)
        ));

        let connection =
            Connection::open(profile.0.join("ledger.sqlite")).expect("reopen rejected database");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let app_id: i64 = connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .expect("read application id");
        let user_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        let user_object_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema \
                 WHERE substr(name, 1, 7) COLLATE NOCASE != 'sqlite_'",
                [],
                |row| row.get(0),
            )
            .expect("count existing user objects");
        assert_eq!(journal, "delete");
        assert_eq!(app_id, 0);
        assert_eq!(user_version, 0);
        assert_eq!(user_object_count, 1);
    }

    #[test]
    fn resumes_a_torn_first_active_marker_promotion() {
        use std::os::unix::fs::FileExt;

        let profile = TempProfile::new();
        let profile_identity = identity("incarnation-test");
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        let initializing = ProfileMarker::initializing(profile_identity.clone());
        write_initial_profile_marker(&owner, &initializing).expect("persist bootstrap marker");
        assert!(!owner.prepare_ledger_file(true).expect("create database"));
        let (connection, generation) =
            initialize_database(&owner.ledger_path(), &profile_identity, 0, true, false)
                .expect("commit initial schema");
        assert_eq!(generation, 1);
        drop(connection);
        owner
            .marker_file()
            .write_at(b"torn active marker", PROFILE_MARKER_SLOT_SIZE as u64)
            .expect("simulate torn active marker write");
        drop(owner);

        let resumed =
            Ledger::open(&profile.0, profile_identity).expect("resume interrupted promotion");
        assert_eq!(resumed.coordinator_generation(), 2);
        drop(resumed);
        let reopened = Ledger::open(&profile.0, identity("incarnation-test"))
            .expect("open after repaired marker");
        assert_eq!(reopened.coordinator_generation(), 3);
    }

    #[test]
    fn resumes_a_torn_ordinary_generation_marker_update() {
        use std::os::unix::fs::FileExt;

        let profile = TempProfile::new();
        let profile_identity = identity("incarnation-test");
        drop(Ledger::create(&profile.0, profile_identity.clone()).expect("create profile"));
        drop(Ledger::open(&profile.0, profile_identity.clone()).expect("advance to generation 2"));

        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        let (connection, generation) =
            initialize_database(&owner.ledger_path(), &profile_identity, 2, false, false)
                .expect("commit generation 3 before marker update");
        assert_eq!(generation, 3);
        drop(connection);
        owner
            .marker_file()
            .write_at(
                b"torn ordinary marker update",
                PROFILE_MARKER_SLOT_SIZE as u64,
            )
            .expect("simulate interrupted active marker write");
        owner.marker_file().sync_all().expect("sync marker damage");
        drop(owner);

        let resumed =
            Ledger::open(&profile.0, profile_identity).expect("recover strictly newer database");
        assert_eq!(resumed.coordinator_generation(), 4);
    }

    #[test]
    fn torn_bootstrap_marker_fails_closed_for_open_and_create() {
        use std::os::unix::fs::FileExt;

        let profile = TempProfile::new();
        let profile_identity = identity("incarnation-test");
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        let initializing = ProfileMarker::initializing(profile_identity.clone());
        let encoded = encode_profile_marker(&initializing).expect("encode initial marker");
        owner
            .marker_file()
            .write_at(&encoded[..24], 0)
            .expect("simulate interrupted initial marker write");
        owner.marker_file().sync_all().expect("sync marker damage");
        drop(owner);

        assert!(matches!(
            Ledger::open(&profile.0, profile_identity.clone()),
            Err(LedgerOpenError::CorruptProfileMarker)
        ));
        assert!(matches!(
            Ledger::create(&profile.0, profile_identity),
            Err(LedgerOpenError::CorruptProfileMarker)
        ));
        assert!(
            !profile.0.join("ledger.sqlite").exists(),
            "neither open nor create may turn an ambiguous marker into a fresh ledger"
        );
        assert_eq!(
            fs::read(profile.0.join("coordinator.lock")).expect("read unchanged marker"),
            &encoded[..24]
        );
    }

    #[test]
    fn explicit_create_does_not_replace_a_torn_marker_after_database_creation() {
        use std::os::unix::fs::FileExt;

        let profile = TempProfile::new();
        let profile_identity = identity("incarnation-test");
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        let initializing = ProfileMarker::initializing(profile_identity.clone());
        let encoded = encode_profile_marker(&initializing).expect("encode initial marker");
        owner
            .marker_file()
            .write_at(&encoded[..24], 0)
            .expect("simulate interrupted initial marker write");
        owner.marker_file().sync_all().expect("sync marker damage");
        assert!(
            !owner
                .prepare_ledger_file(true)
                .expect("create empty database for negative fixture")
        );
        drop(owner);

        assert!(matches!(
            Ledger::create(&profile.0, profile_identity),
            Err(LedgerOpenError::CorruptProfileMarker)
        ));
        let marker = fs::read(profile.0.join("coordinator.lock")).expect("read marker");
        assert_eq!(&marker[..24], &encoded[..24]);
        assert_eq!(
            fs::metadata(profile.0.join("ledger.sqlite")).unwrap().len(),
            0
        );
    }

    #[test]
    fn rejects_wrong_incarnation_without_advancing_generation() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-a")).expect("create profile"));

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-b")),
            Err(LedgerOpenError::ProfileIdentityMismatch)
        ));
        assert_eq!(
            Ledger::open(&profile.0, identity("incarnation-a"))
                .expect("correct incarnation reopens")
                .coordinator_generation(),
            2
        );
    }

    #[test]
    fn refuses_unrelated_sqlite_files_without_changing_their_journal_mode() {
        let profile = TempProfile::new();
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire test lock");
        owner
            .prepare_ledger_file(true)
            .expect("create private database file");
        let connection = Connection::open(owner.ledger_path()).expect("open database");
        connection
            .execute_batch("CREATE TABLE unrelated (value TEXT NOT NULL);")
            .expect("create unrelated schema");
        drop(connection);
        drop(owner);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::MissingProfileMarker)
        ));
        let connection = Connection::open(profile.0.join("ledger.sqlite"))
            .expect("reopen unrelated database for assertion");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read unrelated journal mode");
        assert_eq!(journal, "delete");
        let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name = 'unrelated'",
                [],
                |row| row.get(0),
            )
            .expect("read unrelated table");
        assert_eq!(tables, 1);
    }

    #[test]
    fn refuses_wrong_incarnation_without_changing_journal_mode_or_generation() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-a")).expect("create profile"));

        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("open database for setup");
        let journal: String = connection
            .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
            .expect("switch journal to DELETE");
        assert_eq!(journal, "delete");
        let generation_before: String = connection
            .query_row(
                "SELECT coordinator_generation FROM profile_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("read initial generation");
        drop(connection);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-b")),
            Err(LedgerOpenError::ProfileIdentityMismatch)
        ));

        let connection = Connection::open(database).expect("reopen database for assertions");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let generation_after: String = connection
            .query_row(
                "SELECT coordinator_generation FROM profile_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("read persisted generation");
        assert_eq!(journal, "delete");
        assert_eq!(generation_after, generation_before);
    }

    #[test]
    fn missing_or_empty_ledger_cannot_reset_an_initialized_profile() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));
        let database = profile.0.join("ledger.sqlite");
        fs::remove_file(&database).expect("remove initialized ledger");

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::MissingLedger)
        ));
        assert!(
            !database.exists(),
            "open must not recreate a missing ledger"
        );

        fs::write(&database, []).expect("replace ledger with an empty file");
        fs::set_permissions(&database, fs::Permissions::from_mode(0o600))
            .expect("keep empty ledger private");
        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::IncompatibleDatabase)
        ));
        assert_eq!(fs::metadata(&database).expect("read empty ledger").len(), 0);
    }

    #[test]
    fn opening_a_profile_with_both_state_files_lost_does_not_reinitialize_it() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));
        fs::remove_file(profile.0.join("ledger.sqlite")).expect("remove ledger");
        fs::remove_file(profile.0.join("coordinator.lock")).expect("remove marker and lock");

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::ProfileNotInitialized)
        ));
        assert!(!profile.0.join("ledger.sqlite").exists());
    }

    #[test]
    fn explicit_create_refuses_an_already_initialized_profile() {
        let profile = TempProfile::new();
        let first =
            Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile");
        assert_eq!(first.coordinator_generation(), 1);
        drop(first);

        assert!(matches!(
            Ledger::create(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::ProfileAlreadyInitialized)
        ));
        assert_eq!(
            Ledger::open(&profile.0, identity("incarnation-test"))
                .expect("normal open remains usable")
                .coordinator_generation(),
            2
        );
    }

    #[test]
    fn refuses_database_generation_rollback_before_changing_journal_mode() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));
        drop(Ledger::open(&profile.0, identity("incarnation-test")).expect("advance profile"));

        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("open database for setup");
        connection
            .execute(
                "UPDATE profile_state SET coordinator_generation = '1' WHERE singleton = 1",
                [],
            )
            .expect("roll generation back below marker");
        let journal: String = connection
            .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
            .expect("switch journal to DELETE");
        assert_eq!(journal, "delete");
        drop(connection);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::GenerationRollback {
                database: 1,
                marker: 2
            })
        ));

        let connection = Connection::open(database).expect("reopen database for assertions");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let generation: String = connection
            .query_row(
                "SELECT coordinator_generation FROM profile_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("read rolled back generation");
        assert_eq!(journal, "delete");
        assert_eq!(generation, "1");
    }

    #[test]
    fn damaged_newest_active_marker_cannot_fall_back_to_a_rolled_back_generation() {
        use std::os::unix::fs::FileExt;

        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));
        drop(Ledger::open(&profile.0, identity("incarnation-test")).expect("advance to 2"));
        drop(Ledger::open(&profile.0, identity("incarnation-test")).expect("advance to 3"));

        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("open database for rollback setup");
        connection
            .execute(
                "UPDATE profile_state SET coordinator_generation = '2' WHERE singleton = 1",
                [],
            )
            .expect("restore database generation 2");
        connection
            .query_row("PRAGMA journal_mode = DELETE", [], |row| {
                row.get::<_, String>(0)
            })
            .expect("set rollback fixture journal mode");
        drop(connection);
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        owner
            .marker_file()
            .write_at(&[1], (PROFILE_MARKER_SLOT_SIZE + 100) as u64)
            .expect("corrupt newest marker slot");
        owner.marker_file().sync_all().expect("sync marker damage");
        drop(owner);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::CorruptProfileMarker)
        ));
        let connection = Connection::open(database).expect("reopen rollback fixture");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let generation: String = connection
            .query_row(
                "SELECT coordinator_generation FROM profile_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("read rolled back generation");
        assert_eq!(journal, "delete");
        assert_eq!(generation, "2");
    }

    #[test]
    fn damaged_first_active_marker_cannot_turn_an_empty_database_into_a_new_profile() {
        use std::os::unix::fs::FileExt;

        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));
        fs::remove_file(profile.0.join("ledger.sqlite")).expect("remove initialized database");
        fs::write(profile.0.join("ledger.sqlite"), []).expect("replace it with an empty file");
        fs::set_permissions(
            profile.0.join("ledger.sqlite"),
            fs::Permissions::from_mode(0o600),
        )
        .expect("keep empty database private");
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire profile");
        owner
            .marker_file()
            .write_at(&[1], (PROFILE_MARKER_SLOT_SIZE + 100) as u64)
            .expect("damage first active marker");
        owner.marker_file().sync_all().expect("sync marker damage");
        drop(owner);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::CorruptProfileMarker)
        ));
        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("reopen empty database");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        assert_eq!(journal, "delete");
        drop(connection);
        assert_eq!(
            fs::metadata(database).expect("read empty database").len(),
            0
        );
    }

    #[test]
    fn refuses_matching_database_markers_when_a_required_table_is_missing() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));

        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("open database for setup");
        connection
            .execute_batch("DROP TABLE outbox; PRAGMA journal_mode = DELETE;")
            .expect("remove required table and switch journal mode");
        drop(connection);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::IncompatibleDatabase)
        ));
        let connection = Connection::open(database).expect("reopen database for assertions");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let generation: String = connection
            .query_row(
                "SELECT coordinator_generation FROM profile_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("read generation");
        assert_eq!(journal, "delete");
        assert_eq!(generation, "1");
    }

    #[test]
    fn refuses_a_command_dedup_table_without_its_scoped_primary_key() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));

        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("open database for schema setup");
        connection
            .execute_batch(
                "PRAGMA journal_mode = DELETE;
                 PRAGMA foreign_keys = OFF;
                 DROP TABLE command_results;
                 CREATE TABLE command_results (
                     profile_id TEXT NOT NULL,
                     project_id TEXT NOT NULL,
                     profile_incarnation TEXT NOT NULL,
                     command_id TEXT NOT NULL,
                     intent_projection_json TEXT NOT NULL
                         CHECK (json_valid(intent_projection_json)),
                     response_json TEXT NOT NULL CHECK (json_valid(response_json)),
                     event_cursor TEXT NOT NULL
                 ) STRICT;",
            )
            .expect("replace dedup table without the primary key");
        drop(connection);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::IncompatibleDatabase)
        ));
        let connection = Connection::open(database).expect("reopen altered database");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let generation: String = connection
            .query_row(
                "SELECT coordinator_generation FROM profile_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("read generation");
        assert_eq!(journal, "delete");
        assert_eq!(generation, "1");
    }

    #[test]
    fn schema_preflight_does_not_accept_check_fragments_inside_default_text() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));

        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("open database for schema setup");
        connection
            .execute_batch(
                "PRAGMA journal_mode = DELETE;
                 PRAGMA foreign_keys = OFF;
                 DROP TABLE command_results;
                 CREATE TABLE command_results (
                     profile_id TEXT NOT NULL,
                     project_id TEXT NOT NULL,
                     profile_incarnation TEXT NOT NULL,
                     command_id TEXT NOT NULL,
                     intent_projection_json TEXT NOT NULL
                         DEFAULT 'check(json_valid(intent_projection_json))',
                     response_json TEXT NOT NULL
                         DEFAULT 'check(json_valid(response_json))',
                     event_cursor TEXT NOT NULL,
                     PRIMARY KEY (profile_id, project_id, profile_incarnation, command_id)
                 ) STRICT, WITHOUT ROWID;",
            )
            .expect("replace checks with deceptive default strings");
        drop(connection);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::IncompatibleDatabase)
        ));
        let connection = Connection::open(database).expect("reopen altered database");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let generation: String = connection
            .query_row(
                "SELECT coordinator_generation FROM profile_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("read generation");
        assert_eq!(journal, "delete");
        assert_eq!(generation, "1");
    }

    #[test]
    fn normal_open_rejects_quarantined_profiles_before_mutating_them() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));

        let database = profile.0.join("ledger.sqlite");
        let connection = Connection::open(&database).expect("open database for setup");
        connection
            .execute(
                "UPDATE profile_state SET quarantined = 1, restore_cutoff = 'event-17' \
                 WHERE singleton = 1",
                [],
            )
            .expect("mark profile quarantined");
        connection
            .query_row("PRAGMA journal_mode = DELETE", [], |row| {
                row.get::<_, String>(0)
            })
            .expect("set journal mode for mutation assertion");
        drop(connection);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::ProfileQuarantined)
        ));
        let connection = Connection::open(database).expect("reopen quarantined profile");
        let journal: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal mode");
        let (generation, quarantined, cutoff): (String, i64, String) = connection
            .query_row(
                "SELECT coordinator_generation, quarantined, restore_cutoff \
                 FROM profile_state WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read quarantine state");
        assert_eq!(journal, "delete");
        assert_eq!(generation, "1");
        assert_eq!(quarantined, 1);
        assert_eq!(cutoff, "event-17");
    }

    #[test]
    fn refuses_generation_wraparound() {
        let profile = TempProfile::new();
        drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));
        let owner = ProfileOwnership::acquire(&profile.0).expect("acquire test lock");
        let connection = Connection::open(owner.ledger_path()).expect("open test database");
        connection
            .execute(
                "UPDATE profile_state SET coordinator_generation = ?1 WHERE singleton = 1",
                [u64::MAX.to_string()],
            )
            .expect("set generation to maximum");
        drop(connection);
        drop(owner);

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::GenerationExhausted)
        ));
    }

    #[test]
    fn rejects_a_symlink_database_entry_before_opening_sqlite() {
        use std::os::unix::fs::symlink;

        let profile = TempProfile::new();
        let outside = profile.0.with_extension("outside-ledger");
        fs::write(&outside, b"must not be opened as SQLite")
            .expect("create database symlink target");
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600))
            .expect("make target private");
        symlink(&outside, profile.0.join("ledger.sqlite")).expect("create database symlink");

        assert!(Ledger::open(&profile.0, identity("incarnation-test")).is_err());
    }

    #[test]
    fn rejects_a_symlinked_wal_sidecar_before_opening_sqlite() {
        use std::os::unix::fs::symlink;

        let profile = TempProfile::new();
        let outside = profile.0.with_extension("outside-wal");
        fs::write(&outside, b"not a WAL file").expect("create sidecar target");
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600))
            .expect("make target private");
        symlink(&outside, profile.0.join("ledger.sqlite-wal")).expect("create WAL symlink");

        assert!(Ledger::open(&profile.0, identity("incarnation-test")).is_err());

        fs::remove_file(profile.0.join("ledger.sqlite-wal")).expect("remove WAL symlink");
        fs::remove_file(outside).expect("remove sidecar target");
    }

    #[test]
    fn rejects_an_orphan_wal_sidecar_without_creating_a_new_database() {
        let profile = TempProfile::new();
        let sidecar = profile.0.join("ledger.sqlite-wal");
        fs::write(&sidecar, b"orphan WAL state").expect("create orphan WAL sidecar");
        fs::set_permissions(&sidecar, fs::Permissions::from_mode(0o600))
            .expect("make orphan sidecar private");

        assert!(Ledger::open(&profile.0, identity("incarnation-test")).is_err());
        assert!(!profile.0.join("ledger.sqlite").exists());
    }

    #[test]
    fn rejects_a_hard_linked_database_file() {
        let profile = TempProfile::new();
        let outside = profile.0.with_extension("outside-ledger");
        fs::write(&outside, b"not an independent ledger file").expect("create hard-link target");
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600))
            .expect("make target private");
        fs::hard_link(&outside, profile.0.join("ledger.sqlite"))
            .expect("create database hard link");

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::Ownership(
                OwnershipError::InvalidDatabaseFile
            ))
        ));
    }

    #[test]
    fn rejects_a_database_file_that_is_group_or_world_accessible() {
        let profile = TempProfile::new();
        let database = profile.0.join("ledger.sqlite");
        fs::write(&database, b"not an independent ledger file").expect("create database file");
        fs::set_permissions(&database, fs::Permissions::from_mode(0o644))
            .expect("make database public");

        assert!(matches!(
            Ledger::open(&profile.0, identity("incarnation-test")),
            Err(LedgerOpenError::Ownership(
                OwnershipError::InvalidDatabaseFile
            ))
        ));
    }

    #[test]
    fn rejects_noncanonical_or_zero_persisted_generation() {
        for generation in ["01", "0", "", "18446744073709551616"] {
            let profile = TempProfile::new();
            drop(Ledger::create(&profile.0, identity("incarnation-test")).expect("create profile"));
            let owner = ProfileOwnership::acquire(&profile.0).expect("acquire test lock");
            let connection = Connection::open(owner.ledger_path()).expect("open test database");
            connection
                .execute(
                    "UPDATE profile_state SET coordinator_generation = ?1 WHERE singleton = 1",
                    [generation],
                )
                .expect("set invalid generation");
            drop(connection);
            drop(owner);

            assert!(matches!(
                Ledger::open(&profile.0, identity("incarnation-test")),
                Err(LedgerOpenError::IncompatibleDatabase)
            ));
        }
    }

    #[test]
    fn rejects_identifiers_that_exceed_wire_bounds_or_contain_controls() {
        assert!(matches!(
            ProfileIdentity::new("", "incarnation"),
            Err(LedgerOpenError::InvalidProfileIdentity)
        ));
        assert!(matches!(
            ProfileIdentity::new("profile\ninvalid", "incarnation"),
            Err(LedgerOpenError::InvalidProfileIdentity)
        ));
        assert!(matches!(
            ProfileIdentity::new("profile with spaces", "incarnation"),
            Err(LedgerOpenError::InvalidProfileIdentity)
        ));
        assert!(matches!(
            ProfileIdentity::new("p".repeat(129), "incarnation"),
            Err(LedgerOpenError::InvalidProfileIdentity)
        ));
    }
}
