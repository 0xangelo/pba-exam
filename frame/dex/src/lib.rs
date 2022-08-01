#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod helpers;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use core::ops::AddAssign;

    use crate::helpers::*;
    use codec::FullCodec;
    use frame_support::{
        pallet_prelude::*,
        traits::fungibles::{
            metadata::Mutate as MutateMetadata, Create, Inspect, InspectMetadata, Mutate, Transfer,
        },
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, One, Saturating, Zero},
        ArithmeticError, FixedPointNumber,
    };
    use sp_std::fmt::Debug;

    // ---------------------------------------------------------------------------------------------
    //                                      Config
    // ---------------------------------------------------------------------------------------------

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Unique identifier for an AMM instance.
        type AmmId: Clone
            + Copy
            + CheckedAdd
            + Debug
            + Decode
            + Default
            + Encode
            + FullCodec
            + MaxEncodedLen
            + One
            + PartialEq
            + TypeInfo;

        /// The asset identifier type.
        type AssetId: Clone + Copy + Debug + Decode + Encode + MaxEncodedLen + PartialEq + TypeInfo;

        /// Asset transfer mechanism.
        type Assets: Create<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
            + Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
            + InspectMetadata<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
            + Mutate<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
            + MutateMetadata<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
            + Transfer<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>;

        /// Type of balances for user accounts and AMM reserves.
        type Balance: AddAssign
            + CheckedDiv
            + CheckedMul
            + Clone
            + Copy
            + Debug
            + Decode
            + Encode
            + From<u64>
            + FullCodec
            + MaxEncodedLen
            + One
            + PartialEq
            + Saturating
            + TypeInfo
            + Zero;

        /// Type of unsigned fixed point number used for internal calculations.
        type Decimal: FixedPointNumber;

        /// Default number of decimal digits for AMM share asset.
        type DefaultDecimals: Get<u8>;

        /// Event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The `AccountId` of the pallet.
        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    // ---------------------------------------------------------------------------------------------
    //                                      Pallet Type
    // ---------------------------------------------------------------------------------------------

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // ---------------------------------------------------------------------------------------------
    //                                      Storage
    // ---------------------------------------------------------------------------------------------

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

    /// Mapping from AMM ids to corresponding states.
    #[pallet::storage]
    #[pallet::getter(fn amm_state)]
    pub type AmmStates<T: Config> = StorageMap<_, Blake2_128Concat, T::AmmId, Amm<T>>;

    #[pallet::storage]
    #[pallet::getter(fn amm_count)]
    pub type AmmCount<T: Config> = StorageValue<_, T::AmmId, ValueQuery>;

    /// The share of the pool for each liquidity provider (LP tokens).
    #[pallet::storage]
    #[pallet::getter(fn shares)]
    pub type Shares<T: Config> =
        StorageDoubleMap<_, Twox64Concat, T::AmmId, Blake2_128Concat, T::AccountId, T::Balance>;

    // ---------------------------------------------------------------------------------------------
    //                                      Events
    // ---------------------------------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Emitted when a new Amm is created
        AmmCreated(T::AmmId),
    }

    // ---------------------------------------------------------------------------------------------
    //                                      Errors
    // ---------------------------------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// Raised when an operation targets a nonexistent AMM.
        InvalidAmmId,
        /// Raised when failing to create a new asset type for LP shares.
        InvalidShareAsset,
        /// Raised when trying to provide liquidity with non-equivalent values of the two assets in the pool.
        NonEquivalentValue,
    }

    // ---------------------------------------------------------------------------------------------
    //                                      Extrinsics
    // ---------------------------------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000)]
        pub fn create_amm(
            origin: OriginFor<T>,
            base_asset: T::AssetId,
            quote_asset: T::AssetId,
            share_asset: T::AssetId,
            fees_bps: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let amm_id = Self::amm_count();
            let amm_state = Amm {
                base_asset,
                base_reserves: Zero::zero(),
                quote_asset,
                quote_reserves: Zero::zero(),
                share_asset,
                total_shares: Zero::zero(),
                fees_bps,
            };

            let amm_account = Self::amm_account(&amm_id);
            T::Assets::create(
                share_asset,
                amm_account,
                true,
                One::one(), // Any share amount is fair game
            )
            .map_err(|_| Error::<T>::InvalidShareAsset)?;

            // Mutating metadata requires a deposit, so calling this function with the just-created
            // `amm_account` raises an "InsufficientBalance" error.
            // <T::Assets as MutateMetadata<T::AccountId>>::set(
            //     share_asset,
            //     &amm_account,
            //     (*b"").into(),
            //     (*b"").into(),
            //     T::DefaultDecimals::get(),
            // )?;

            AmmCount::<T>::set(
                amm_id
                    .checked_add(&One::one())
                    .ok_or(ArithmeticError::Overflow)?,
            );
            AmmStates::<T>::insert(amm_id, amm_state);

            Self::deposit_event(Event::<T>::AmmCreated(amm_id));
            Ok(())
        }

        #[pallet::weight(1_000)]
        pub fn provide_liquidity(
            origin: OriginFor<T>,
            amm_id: T::AmmId,
            base_amount: T::Balance,
            quote_amount: T::Balance,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let mut state = Self::amm_state(&amm_id).ok_or(Error::<T>::InvalidAmmId)?;

            let shares = if state.total_shares.is_zero() {
                let unit: T::Balance = 10_u64
                    .saturating_pow(T::DefaultDecimals::get() as u32)
                    .into();
                unit.saturating_mul(100_u64.into())
            } else {
                let share1 = state
                    .total_shares
                    .try_mul(&base_amount)?
                    .try_div(&state.base_reserves)?;
                let share2 = state
                    .total_shares
                    .try_mul(&quote_amount)?
                    .try_div(&state.quote_reserves)?;

                // This might be too strict. By the time the extrinsic is received and executed by
                // the node, the reserves may have changed (even if slightly) from the when the
                // caller calculated the amount of each asset.
                ensure!(share1 == share2, Error::<T>::NonEquivalentValue);

                share1
            };

            T::Assets::transfer(
                state.base_asset,
                &caller,
                &Self::amm_account(&amm_id),
                base_amount,
                false,
            )?;

            T::Assets::transfer(
                state.quote_asset,
                &caller,
                &Self::amm_account(&amm_id),
                quote_amount,
                false,
            )?;

            T::Assets::mint_into(state.share_asset, &caller, shares)?;

            state.base_reserves += base_amount;
            state.quote_reserves += quote_amount;
            state.total_shares += shares;

            AmmStates::<T>::insert(&amm_id, state);

            Ok(())
        }
    }

    // ---------------------------------------------------------------------------------------------
    //                                      Helpers
    // ---------------------------------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        fn amm_account(amm_id: &T::AmmId) -> T::AccountId {
            T::PalletId::get().into_sub_account_truncating(amm_id)
        }
    }
}
