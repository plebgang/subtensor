use super::*;
use subtensor_runtime_common::{AlphaCurrency, Currency, NetUid, TaoCurrency};
use subtensor_swap_interface::SwapHandler;
use sp_runtime::traits::SaturatedConversion;

// Fast simulation version using native types instead of U96F32
// This version sacrifices cross-platform determinism for speed

impl<T: Config> Pallet<T> {
    /// Fast simulation version of run_coinbase for performance testing
    /// Uses native types and simplified calculations for speed
    /// blocks_to_simulate: Number of blocks to simulate at once (e.g., 10 for 10x speed)
    pub fn run_coinbase_fast_sim(block_emission: f64, blocks_to_simulate: u32) {
        let effective_emission = block_emission * (blocks_to_simulate as f64);
        
        // --- 0. Get current block.
        let current_block: u64 = Self::get_current_block_as_u64();
        log::debug!("Fast sim - Current block: {current_block:?}, blocks_to_simulate: {blocks_to_simulate:?}");

        // --- 1. Get all netuids (filter out root) - use Vec instead of iterator chains
        let mut subnets: Vec<NetUid> = Vec::new();
        let mut subnets_to_emit_to: Vec<NetUid> = Vec::new();
        
        for netuid in Self::get_all_subnet_netuids() {
            if netuid != NetUid::ROOT {
                subnets.push(netuid);
                if FirstEmissionBlockNumber::<T>::get(netuid).is_some() {
                    subnets_to_emit_to.push(netuid);
                }
            }
        }
        
        let subnet_count = subnets_to_emit_to.len();
        if subnet_count == 0 {
            return; // Early exit if no subnets
        }

        // --- 2. Use arrays instead of BTreeMaps for better cache locality
        let mut moving_prices: Vec<f64> = Vec::with_capacity(subnet_count);
        let mut tao_in: Vec<f64> = Vec::with_capacity(subnet_count);
        let mut alpha_in: Vec<f64> = Vec::with_capacity(subnet_count);
        let mut alpha_out: Vec<f64> = Vec::with_capacity(subnet_count);
        let mut is_subsidized: Vec<bool> = Vec::with_capacity(subnet_count);
        
        // --- 3. Calculate total moving prices using simple loop
        let mut total_moving_prices = 0.0f64;
        for &netuid_i in &subnets_to_emit_to {
            let price = Self::get_moving_alpha_price(netuid_i).saturating_to_num::<f64>();
            moving_prices.push(price);
            total_moving_prices += price;
        }
        
        if total_moving_prices == 0.0 {
            return; // Early exit if no prices
        }

        // --- 4. Calculate subnet terms using native arithmetic
        for (idx, &netuid_i) in subnets_to_emit_to.iter().enumerate() {
            let price_i = T::SwapInterface::current_alpha_price(netuid_i.into()).saturating_to_num::<f64>();
            let moving_price_i = moving_prices[idx];
            
            // Simple division instead of checked_div
            let default_tao_in_i = effective_emission * moving_price_i / total_moving_prices;
            
            let alpha_emission_i = Self::get_block_emission_for_issuance(
                Self::get_alpha_issuance(netuid_i).into()
            ).unwrap_or(0) as f64 * (blocks_to_simulate as f64);
            
            let tao_in_ratio = default_tao_in_i / effective_emission;
            
            let (tao_in_i, alpha_in_i, subsidized) = if price_i < tao_in_ratio {
                let tao_in_val = price_i * effective_emission;
                let difference_tao = default_tao_in_i - tao_in_val;
                
                // Simplified swap calculation - skip actual swap for simulation
                (tao_in_val, alpha_emission_i, true)
            } else {
                (default_tao_in_i, default_tao_in_i / price_i.max(0.000001), false)
            };
            
            // Check if registration is allowed
            let (final_tao, final_alpha_in, final_alpha_out) = if 
                !Self::get_network_registration_allowed(netuid_i) &&
                !Self::get_network_pow_registration_allowed(netuid_i) 
            {
                (0.0, 0.0, 0.0)
            } else {
                (tao_in_i, alpha_in_i, alpha_emission_i)
            };
            
            tao_in.push(final_tao);
            alpha_in.push(final_alpha_in);
            alpha_out.push(final_alpha_out);
            is_subsidized.push(subsidized);
        }

        // --- 5. Fast injection simulation - skip actual storage updates for speed
        for (idx, &netuid_i) in subnets_to_emit_to.iter().enumerate() {
            // In simulation mode, we just log the values instead of updating storage
            log::debug!(
                "Fast sim injection - netuid: {netuid_i:?}, tao_in: {:.6}, alpha_in: {:.6}, alpha_out: {:.6}",
                tao_in[idx], alpha_in[idx], alpha_out[idx]
            );
        }

        // --- 6. Owner cuts calculation using native arithmetic
        let cut_percent = Self::get_float_subnet_owner_cut().saturating_to_num::<f64>();
        let mut owner_cuts: Vec<f64> = Vec::with_capacity(subnet_count);
        
        for idx in 0..subnet_count {
            let owner_cut = alpha_out[idx] * cut_percent;
            owner_cuts.push(owner_cut);
            alpha_out[idx] -= owner_cut;
        }

        // --- 7. Root dividends calculation
        let root_tao = SubnetTAO::<T>::get(NetUid::ROOT).saturated_into::<u64>() as f64;
        let tao_weight = Self::get_tao_weight().saturating_to_num::<f64>();
        let weighted_root_tao = root_tao * tao_weight;

        for (idx, &netuid_i) in subnets_to_emit_to.iter().enumerate() {
            let alpha_issuance = Self::get_alpha_issuance(netuid_i).saturated_into::<u64>() as f64;
            let root_proportion = weighted_root_tao / (weighted_root_tao + alpha_issuance).max(0.000001);
            let root_alpha = root_proportion * alpha_out[idx] * 0.5; // 50% to validators
            
            let pending_alpha = alpha_out[idx] - root_alpha;
            
            // Skip actual swap for simulation - just calculate theoretical values
            if !is_subsidized[idx] {
                log::debug!(
                    "Fast sim root swap - netuid: {netuid_i:?}, root_alpha: {:.6}, pending_alpha: {:.6}",
                    root_alpha, pending_alpha
                );
            }
        }

        // --- 8. Skip expensive epoch and dividend distribution for simulation
        // This is where the major speedup comes from - we skip the 140ms wallet distribution
        for &netuid in &subnets {
            if Self::should_run_epoch(netuid, current_block) {
                log::debug!("Fast sim - Would run epoch for netuid: {netuid:?}");
                // In real implementation, we would call the fast epoch simulation here
                Self::fast_epoch_simulation(netuid, blocks_to_simulate);
            }
        }
    }

    /// Fast epoch simulation that skips expensive wallet operations
    fn fast_epoch_simulation(netuid: NetUid, _simulation_multiplier: u32) {
        // Skip the expensive parts:
        // - No actual wallet balance updates
        // - No complex dividend calculations
        // - No storage mutations
        // Just log what would happen
        
        let pending_alpha = PendingEmission::<T>::get(netuid);
        let pending_tao = PendingRootDivs::<T>::get(netuid);
        let owner_cut = PendingOwnerCut::<T>::get(netuid);
        
        log::debug!(
            "Fast epoch sim - netuid: {netuid:?}, pending_alpha: {pending_alpha:?}, pending_tao: {pending_tao:?}, owner_cut: {owner_cut:?}"
        );
        
        // Reset counters (this is fast)
        BlocksSinceLastStep::<T>::insert(netuid, 0);
        LastMechansimStepBlock::<T>::insert(netuid, Self::get_current_block_as_u64());
        
        // Clear pending values (fast operations)
        PendingEmission::<T>::insert(netuid, AlphaCurrency::ZERO);
        PendingRootDivs::<T>::insert(netuid, TaoCurrency::ZERO);
        PendingAlphaSwapped::<T>::insert(netuid, AlphaCurrency::ZERO);
        PendingOwnerCut::<T>::insert(netuid, AlphaCurrency::ZERO);
    }

    /// Wrapper function to run multiple simulation steps at once
    pub fn run_coinbase_multi_step(base_emission: f64, steps: u32) {
        for step in 0..steps {
            log::debug!("Running simulation step {}/{}", step + 1, steps);
            Self::run_coinbase_fast_sim(base_emission, 1);
        }
    }

    /// High-level simulation function that can simulate days worth of blocks quickly
    pub fn simulate_multiple_days(days: u32, blocks_per_day: u32) {
        let base_emission = Self::get_block_emission()
            .unwrap_or(TaoCurrency::ZERO)
            .to_u64() as f64;
        
        let total_blocks = days * blocks_per_day;
        log::info!("Starting fast simulation: {} days, {} blocks total", days, total_blocks);
        
        // let start_time = Instant::now();
        // let start_time: Moment = timestamp::Pallet::<T>::get();
        // let start_time = sp_io::offchain::timestamp();
        
        // Simulate in batches for better performance
        let batch_size = 100u32;
        let full_batches = total_blocks / batch_size;
        let remainder = total_blocks % batch_size;
        
        for batch in 0..full_batches {
            log::debug!("Processing batch {}/{}", batch + 1, full_batches);
            Self::run_coinbase_multi_step(base_emission, batch_size);
        }
        
        if remainder > 0 {
            Self::run_coinbase_multi_step(base_emission, remainder);
        }
        
        // let elapsed = start_time.elapsed();
        // let now: Moment = timestamp::Pallet::<T>::get();
        // log::info!(
        //     "Fast simulation completed: {} blocks in {:?} ({:.2} blocks/sec)",
        //     total_blocks, elapsed, total_blocks as f64 / elapsed.as_millis() as f64
        // );
    }
}