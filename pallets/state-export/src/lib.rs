#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{vec::Vec, string::String};
use codec::{Decode, Encode};
use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    pallet_prelude::*,
    traits::{Get, StorageInstance},
    weights::Weight,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::{
    traits::{Block as BlockT, Header as HeaderT, Saturating, Zero},
    SaturatedConversion,
};
use sp_std::collections::btree_map::BTreeMap;
use state_export_runtime_api::{ChainState, StateExportConfig, StorageEntry};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// The maximum size of exported state data
        #[pallet::constant]
        type MaxStateSize: Get<u32>;
        
        /// The maximum number of storage prefixes to track
        #[pallet::constant]
        type MaxStoragePrefixes: Get<u32>;
    }

    /// Storage for the state export configuration
    #[pallet::storage]
    #[pallet::getter(fn state_export_config)]
    pub type StateExportConfiguration<T: Config> = StorageValue<_, StateExportConfig, ValueQuery>;

    /// Storage for tracking the last exported block
    #[pallet::storage]
    #[pallet::getter(fn last_exported_block)]
    pub type LastExportedBlock<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// Storage for caching exported states (optional, for debugging)
    #[pallet::storage]
    #[pallet::getter(fn cached_states)]
    pub type CachedStates<T: Config> = StorageMap<_, Blake2_128Concat, T::BlockNumber, ChainState>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// State export configuration updated
        StateExportConfigUpdated { config: StateExportConfig },
        /// State exported successfully
        StateExported { block_number: T::BlockNumber, entries_count: u32 },
        /// State export failed
        StateExportFailed { block_number: T::BlockNumber, reason: Vec<u8> },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// State export is disabled
        StateExportDisabled,
        /// Invalid configuration
        InvalidConfiguration,
        /// State too large to export
        StateTooLarge,
        /// Export failed
        ExportFailed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Configure state export settings (requires root)
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn configure_state_export(
            origin: OriginFor<T>,
            config: StateExportConfig,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            
            // Validate configuration
            ensure!(
                config.storage_prefixes.len() <= T::MaxStoragePrefixes::get() as usize,
                Error::<T>::InvalidConfiguration
            );
            
            StateExportConfiguration::<T>::put(&config);
            Self::deposit_event(Event::StateExportConfigUpdated { config });
            
            Ok(().into())
        }

        /// Manually trigger state export (requires root)
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(100_000, 0))]
        pub fn export_state(
            origin: OriginFor<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            
            let config = Self::state_export_config();
            ensure!(config.enabled, Error::<T>::StateExportDisabled);
            
            let current_block = frame_system::Pallet::<T>::block_number();
            match Self::perform_state_export(current_block) {
                Ok(entries_count) => {
                    Self::deposit_event(Event::StateExported { 
                        block_number: current_block, 
                        entries_count 
                    });
                    Ok(().into())
                },
                Err(reason) => {
                    Self::deposit_event(Event::StateExportFailed { 
                        block_number: current_block, 
                        reason: reason.into() 
                    });
                    Err(Error::<T>::ExportFailed.into())
                }
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block_number: T::BlockNumber) {
            let config = Self::state_export_config();
            
            if config.enabled && config.export_frequency > 0 {
                let block_num: u32 = block_number.saturated_into();
                if block_num % config.export_frequency == 0 {
                    if let Ok(entries_count) = Self::perform_state_export(block_number) {
                        Self::deposit_event(Event::StateExported { 
                            block_number, 
                            entries_count 
                        });
                        LastExportedBlock::<T>::put(block_number);
                    } else {
                        Self::deposit_event(Event::StateExportFailed { 
                            block_number, 
                            reason: b"Auto export failed".to_vec() 
                        });
                    }
                }
            }
        }
    }

    impl<T: Config> Pallet<T> {
        /// Perform the actual state export
        pub fn perform_state_export(block_number: T::BlockNumber) -> Result<u32, &'static str> {
            let config = Self::state_export_config();
            let current_block_hash = frame_system::Pallet::<T>::block_hash(block_number);
            
            // Collect storage entries
            let storage_entries = if config.include_all_storage {
                Self::collect_all_storage()
            } else {
                Self::collect_storage_with_prefixes(&config.storage_prefixes)
            };
            
            // Check size limits
            let entries_count = storage_entries.len() as u32;
            if entries_count > T::MaxStateSize::get() {
                return Err("State too large");
            }
            
            // Create chain state object
            let chain_state = ChainState {
                block_number: block_number.saturated_into(),
                block_hash: current_block_hash.encode(),
                storage_entries,
                timestamp: Self::get_current_timestamp(),
            };
            
            // Cache the state (optional)
            CachedStates::<T>::insert(block_number, &chain_state);
            
            // Serialize to JSON and write to file (in offchain context)
            if let Ok(json_data) = serde_json::to_string_pretty(&chain_state) {
                Self::write_state_to_file(&config.export_path, &json_data, block_number.saturated_into());
            }
            
            Ok(entries_count)
        }
        
        /// Collect all storage entries
        fn collect_all_storage() -> Vec<StorageEntry> {
            use sp_io::storage;
            let mut entries = Vec::new();
            
            // Get all storage keys
            let keys = storage::next_key(&[]).unwrap_or_default();
            let mut current_key = Some(keys);
            
            while let Some(key) = current_key {
                if let Some(value) = storage::get(&key) {
                    entries.push(StorageEntry {
                        key: key.clone(),
                        value: Some(value),
                    });
                }
                current_key = storage::next_key(&key);
            }
            
            entries
        }
        
        /// Collect storage entries with specific prefixes
        fn collect_storage_with_prefixes(prefixes: &[Vec<u8>]) -> Vec<StorageEntry> {
            use sp_io::storage;
            let mut entries = Vec::new();
            
            for prefix in prefixes {
                // Start iteration from the prefix
                let mut current_key = storage::next_key(prefix);
                
                while let Some(key) = current_key {
                    // Check if key still has the prefix
                    if !key.starts_with(prefix) {
                        break;
                    }
                    
                    if let Some(value) = storage::get(&key) {
                        entries.push(StorageEntry {
                            key: key.clone(),
                            value: Some(value),
                        });
                    }
                    
                    current_key = storage::next_key(&key);
                }
            }
            
            entries
        }
        
        /// Get current timestamp
        fn get_current_timestamp() -> u64 {
            use sp_io::offchain;
            offchain::timestamp().unix_millis()
        }
        
        /// Write state data to file using offchain storage
        fn write_state_to_file(export_path: &[u8], json_data: &str, block_number: u32) {
            use sp_io::offchain;
            
            // Convert path to string
            let path_str = core::str::from_utf8(export_path).unwrap_or("/tmp/chain_state");
            
            // Create filename with block number
            let filename = alloc::format!("{}/chain_state_block_{}.json", path_str, block_number);
            
            // Write to offchain storage
            let storage_key = alloc::format!("state_export::{}", block_number);
            offchain::local_storage_set(
                sp_core::offchain::StorageKind::PERSISTENT,
                storage_key.as_bytes(),
                json_data.as_bytes(),
            );
            
            // Log the export (in a real implementation, you might use offchain HTTP requests
            // to write to actual files or send to external services)
            log::info!("State exported to storage key: {} (simulated file: {})", storage_key, filename);
        }
    }
}

/// Implementation of the runtime API
impl<T: Config> state_export_runtime_api::StateExportApi<T::Block> for Pallet<T> {
    fn get_chain_state() -> ChainState {
        let current_block = frame_system::Pallet::<T>::block_number();
        Self::perform_state_export(current_block)
            .map(|_| {
                Self::cached_states(current_block).unwrap_or_default()
            })
            .unwrap_or_default()
    }
    
    fn get_chain_state_at_block(block_number: u32) -> Option<ChainState> {
        let block_num = T::BlockNumber::from(block_number);
        Self::cached_states(block_num)
    }
    
    fn get_storage_with_prefixes(prefixes: Vec<Vec<u8>>) -> Vec<StorageEntry> {
        Self::collect_storage_with_prefixes(&prefixes)
    }
    
    fn get_state_export_config() -> StateExportConfig {
        Self::state_export_config()
    }
    
    fn set_state_export_config(config: StateExportConfig) -> bool {
        StateExportConfiguration::<T>::put(&config);
        true
    }
    
    fn export_state_to_json() -> bool {
        let current_block = frame_system::Pallet::<T>::block_number();
        Self::perform_state_export(current_block).is_ok()
    }
}

/// Default implementation for ChainState
impl Default for ChainState {
    fn default() -> Self {
        Self {
            block_number: 0,
            block_hash: Vec::new(),
            storage_entries: Vec::new(),
            timestamp: 0,
        }
    }
}

/// Default implementation for StateExportConfig
impl Default for StateExportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            export_path: b"/tmp/chain_state".to_vec(),
            export_frequency: 100, // Export every 100 blocks
            include_all_storage: false,
            storage_prefixes: Vec::new(),
        }
    }
}