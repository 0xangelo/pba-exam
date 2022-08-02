use crate::{helpers::TryMul, Config};
use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};
use sp_runtime::{traits::Zero, ArithmeticError};

/// The state of a particular AMM.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct Amm<T: Config> {
    pub base_asset: T::AssetId,
    pub base_reserves: T::Balance,
    pub quote_asset: T::AssetId,
    pub quote_reserves: T::Balance,
    pub share_asset: T::AssetId,
    pub total_shares: T::Balance,
    pub fees_bps: T::Balance,
}

impl<T: Config> Amm<T> {
    /// Compute the invariant for this AMM, i.e., the `K` in `x * y = K`.
    pub fn get_k(&self) -> Result<T::Balance, ArithmeticError> {
        self.base_reserves.try_mul(&self.quote_reserves)
    }

    pub fn is_initialized(&self) -> Result<bool, ArithmeticError> {
        Ok(!self.get_k()?.is_zero())
    }
}

/// For indicating the input to swaps
#[derive(Clone, Debug, Decode, Encode, MaxEncodedLen, PartialEq, Eq, TypeInfo)]
pub enum AssetType {
    Base,
    Quote,
}
