#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod helpers;
mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use crate::{helpers::*, types::*};
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
        traits::{
            AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Saturating,
            Zero,
        },
        ArithmeticError,
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
        type Balance: CheckedAdd
            + CheckedDiv
            + CheckedMul
            + CheckedSub
            + Clone
            + Copy
            + Debug
            + Decode
            + Encode
            + From<u64>
            + FullCodec
            + MaxEncodedLen
            + One
            + Ord
            + PartialEq
            + Saturating
            + TypeInfo
            + Zero;

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
        /// Emitted when a new Amm is created.
        AmmCreated(T::AmmId),
        /// Emitted when a user adds liquidity to an AMM.
        LiquidityAdded {
            amm_id: T::AmmId,
            user: T::AccountId,
            shares: T::Balance,
        },
        /// Emitted when a user withdraws liquidity from an AMM.
        LiquidityRemoved {
            amm_id: T::AmmId,
            user: T::AccountId,
            shares: T::Balance,
        },
        /// Emitted when a user swaps against an AMM.
        Swapped {
            user: T::AccountId,
            amm_id: T::AmmId,
            asset_type: AssetType,
            input_amount: T::Balance,
            output_amount: T::Balance,
        },
    }

    // ---------------------------------------------------------------------------------------------
    //                                      Errors
    // ---------------------------------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// Raised when an operation targets a nonexistent AMM.
        InvalidAmmId,
        /// Raised when trying to withdraw more LP shares than a user has in their account.
        InvalidShareAmount,
        /// Raised when failing to create a new asset type for LP shares.
        InvalidShareAsset,
        /// Raised when trying to provide liquidity with non-equivalent values of the two assets in
        /// the pool.
        NonEquivalentValue,
        /// Raised when swap output is below the minimum required by a user.
        SlippageExceeded,
        /// Raised when trying to swap a zero amount of asset.
        ZeroAmount,
        /// Raised when interacting with an uninitialized AMM while the operation requires
        /// otherwise.
        ZeroLiquidity,
    }

    // ---------------------------------------------------------------------------------------------
    //                                      Extrinsics
    // ---------------------------------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new AMM.
        ///
        /// The caller has to specify:
        /// - `base_asset`: the asset id of the first one in the pool pair
        /// - `quote_asset`: the asset id of the second one in the pool pair
        /// - `share_asset`: the asset id of the liquidity provider token to be created and managed
        ///   by this AMM
        /// - `fees_bps`: the share of input asset to be subtracted as fees for liquidity providers
        ///   in the `swap` extrinsic.
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

        /// Provide liquidity to an AMM pool.
        ///
        /// The caller must specify the following parameters
        /// - `amm_id`: the if of the AMM to add assets to
        /// - `base_amount`: the amount of base asset to add
        /// - `quote_amount`: the amount of quote asset to add
        ///
        /// The caller has to make sure that the ratio of `base_amount` to `quote_amount` matches
        /// exactly the proportion of these assets in the pool, if it already has liquidity. If the
        /// caller is the first to provide liquidity, it sets the ratio of these assets and the
        /// implied invariant.
        ///
        /// The pallet mints LP 'shares' as the asset which was created during the call to
        /// `create_amm`. The asset amount represents the LP's share of the pool's liquidity, which
        /// accrue rewards through trading fees as traders use the `swap` extrinsic.
        #[pallet::weight(1_000)]
        pub fn provide_liquidity(
            origin: OriginFor<T>,
            amm_id: T::AmmId,
            base_amount: T::Balance,
            quote_amount: T::Balance,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let mut state = Self::try_get_amm_state(&amm_id)?;

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

            state.base_reserves = state.base_reserves.try_add(&base_amount)?;
            state.quote_reserves = state.quote_reserves.try_add(&quote_amount)?;
            state.total_shares = state.total_shares.try_add(&shares)?;

            AmmStates::<T>::insert(&amm_id, state);

            Self::deposit_event(Event::<T>::LiquidityAdded {
                amm_id,
                user: caller,
                shares,
            });

            Ok(())
        }

        /// Withdraw liquidity from an AMM's pool.
        ///
        /// The caller must specify the following arguments
        /// - `amm_id`: the id of the AMM
        /// - `amount`: quantity of LP shares to burn from the caller's account in order to return
        ///   its corresponding share of the pool's liquidity.
        #[pallet::weight(1_000)]
        pub fn withdraw(
            origin: OriginFor<T>,
            amm_id: T::AmmId,
            amount: T::Balance,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let mut amm_state = Self::try_get_amm_state(&amm_id)?;

            T::Assets::burn_from(amm_state.share_asset, &caller, amount)
                .map_err(|_| Error::<T>::InvalidShareAmount)?;

            let base_amount = amount
                .try_mul(&amm_state.base_reserves)?
                .try_div(&amm_state.total_shares)?;
            let quote_amount = amount
                .try_mul(&amm_state.quote_reserves)?
                .try_div(&amm_state.total_shares)?;

            let amm_account = Self::amm_account(&amm_id);
            T::Assets::transfer(
                amm_state.base_asset,
                &amm_account,
                &caller,
                base_amount,
                false,
            )?;
            T::Assets::transfer(
                amm_state.quote_asset,
                &amm_account,
                &caller,
                quote_amount,
                false,
            )?;

            amm_state.total_shares = amm_state.total_shares.try_sub(&amount)?;
            amm_state.base_reserves = amm_state.base_reserves.try_sub(&base_amount)?;
            amm_state.quote_reserves = amm_state.quote_reserves.try_sub(&quote_amount)?;

            AmmStates::<T>::insert(&amm_id, amm_state);

            Self::deposit_event(Event::<T>::LiquidityRemoved {
                amm_id,
                user: caller,
                shares: amount,
            });

            Ok(())
        }

        /// Swap either token through the AMM.
        ///
        /// The caller must specify the following arguments
        /// - `amm_id`: the id of the AMM to swap against
        /// - `asset_type`: which of the two asset types in the pool to use as input
        /// - `input_amount`: amount of input asset to add to the AMM
        /// - `output_min`: the minimum amount of the opposite asset to get in return. Prevents
        ///   against slippage.
        #[pallet::weight(1_000)]
        pub fn swap(
            origin: OriginFor<T>,
            amm_id: T::AmmId,
            asset_type: AssetType,
            input_amount: T::Balance,
            output_min: T::Balance,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            ensure!(!input_amount.is_zero(), Error::<T>::ZeroAmount);

            let mut amm_state = Self::try_get_amm_state(&amm_id)?;
            ensure!(amm_state.is_initialized()?, Error::<T>::ZeroLiquidity);

            let output_amount = match asset_type {
                AssetType::Base => {
                    Self::get_quote_estimate_given_base_amount(&amm_state, input_amount)?
                }
                AssetType::Quote => {
                    Self::get_base_estimate_given_quote_amount(&amm_state, input_amount)?
                }
            };
            ensure!(output_amount > output_min, Error::<T>::SlippageExceeded);

            let amm_account = Self::amm_account(&amm_id);
            match asset_type {
                AssetType::Base => {
                    T::Assets::transfer(
                        amm_state.base_asset,
                        &caller,
                        &amm_account,
                        input_amount,
                        false,
                    )?;
                    T::Assets::transfer(
                        amm_state.quote_asset,
                        &amm_account,
                        &caller,
                        output_amount,
                        false,
                    )?;

                    amm_state.base_reserves = amm_state.base_reserves.try_add(&input_amount)?;
                    amm_state.quote_reserves = amm_state.quote_reserves.try_sub(&output_amount)?;
                }
                AssetType::Quote => {
                    T::Assets::transfer(
                        amm_state.base_asset,
                        &amm_account,
                        &caller,
                        output_amount,
                        false,
                    )?;
                    T::Assets::transfer(
                        amm_state.quote_asset,
                        &caller,
                        &amm_account,
                        input_amount,
                        false,
                    )?;

                    amm_state.base_reserves = amm_state.base_reserves.try_sub(&output_amount)?;
                    amm_state.quote_reserves = amm_state.quote_reserves.try_add(&input_amount)?;
                }
            }

            AmmStates::<T>::insert(&amm_id, amm_state);

            Self::deposit_event(Event::<T>::Swapped {
                user: caller,
                amm_id,
                asset_type,
                input_amount,
                output_amount,
            });

            Ok(())
        }
    }

    // ---------------------------------------------------------------------------------------------
    //                                      Helpers
    // ---------------------------------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        fn try_get_amm_state(amm_id: &T::AmmId) -> Result<Amm<T>, DispatchError> {
            Self::amm_state(amm_id).ok_or_else(|| Error::<T>::InvalidAmmId.into())
        }

        fn amm_account(amm_id: &T::AmmId) -> T::AccountId {
            T::PalletId::get().into_sub_account_truncating(amm_id)
        }

        fn get_base_estimate_given_quote_amount(
            amm_state: &Amm<T>,
            amount: T::Balance,
        ) -> Result<T::Balance, DispatchError> {
            let full_bps: T::Balance = 10_000_u64.into();
            let net_amount = full_bps
                .try_sub(&amm_state.fees_bps)?
                .try_mul(&amount)?
                .try_div(&full_bps)?;

            let quote_reserves_after = amm_state.quote_reserves.try_add(&net_amount)?;
            let base_reserves_after = amm_state.get_k()?.try_div(&quote_reserves_after)?;

            // Ensure reserves are not depleted
            let mut base_amount = amm_state.base_reserves.try_sub(&base_reserves_after)?;
            if base_reserves_after.is_zero() {
                base_amount = base_amount.try_sub(&One::one())?;
            }

            Ok(base_amount)
        }

        fn get_quote_estimate_given_base_amount(
            amm_state: &Amm<T>,
            amount: T::Balance,
        ) -> Result<T::Balance, DispatchError> {
            let full_bps: T::Balance = 10_000_u64.into();
            let net_amount = full_bps
                .try_sub(&amm_state.fees_bps)?
                .try_mul(&amount)?
                .try_div(&full_bps)?;

            let base_reserves_after = amm_state.base_reserves.try_add(&net_amount)?;
            let quote_reserves_after = amm_state.get_k()?.try_div(&base_reserves_after)?;

            // Ensure reserves are not depleted
            let mut quote_amount = amm_state.quote_reserves.try_sub(&quote_reserves_after)?;
            if quote_reserves_after.is_zero() {
                quote_amount = quote_amount.try_sub(&One::one())?;
            }

            Ok(quote_amount)
        }
    }
}
