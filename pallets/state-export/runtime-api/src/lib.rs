#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_runtime::traits::Block as BlockT;
use sp_std::collections::btree_map::BTreeMap;

/// Represents a storage key-value pair
#[derive(Encode, Decode, Serialize, Deserialize, Clone, Debug)]
#[freeze_struct("0.1.0")]
pub struct StorageEntry {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
}

/// Represents the complete chain state at a specific block
#[derive(Encode, Decode, Serialize, Deserialize, Clone, Debug)]
#[freeze_struct("0.1.0")]
pub struct ChainState {
    pub block_number: u32,
    pub block_hash: Vec<u8>,
    pub storage_entries: Vec<StorageEntry>,
    pub timestamp: u64,
}

/// Configuration for state export
#[derive(Encode, Decode, Serialize, Deserialize, Clone, Debug)]
#[freeze_struct("0.1.0")]
pub struct StateExportConfig {
    pub enabled: bool,
    pub export_path: Vec<u8>, // Path as bytes to avoid std dependency
    pub export_frequency: u32, // Export every N blocks
    pub include_all_storage: bool,
    pub storage_prefixes: Vec<Vec<u8>>, // Only export storage with these prefixes
}

sp_api::decl_runtime_apis! {
    /// Runtime API for exporting chain state
    pub trait StateExportApi {
        /// Get the current chain state
        fn get_chain_state() -> ChainState;
        
        /// Get chain state for a specific block
        fn get_chain_state_at_block(block_number: u32) -> Option<ChainState>;
        
        /// Get storage entries with specific prefixes
        fn get_storage_with_prefixes(prefixes: Vec<Vec<u8>>) -> Vec<StorageEntry>;
        
        /// Get the current state export configuration
        fn get_state_export_config() -> StateExportConfig;
        
        /// Set the state export configuration (requires root)
        fn set_state_export_config(config: StateExportConfig) -> bool;
        
        /// Export current state to JSON (returns success status)
        fn export_state_to_json() -> bool;
    }
}