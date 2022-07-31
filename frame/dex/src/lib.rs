#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use codec::FullCodec;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{CheckedAdd, One, Zero},
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
        /// Type of balances for user accounts and AMM reserves.
        type Balance: Clone
            + Debug
            + Decode
            + Encode
            + FullCodec
            + MaxEncodedLen
            + PartialEq
            + TypeInfo
            + Zero;
        /// Event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
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
        pub total_shares: T::Balance,
        pub base_reserves: T::Balance,
        pub quote_reserves: T::Balance,
        pub fees_bps: T::Balance,
    }

    /// Mapping from AMM ids to corresponding states.
    #[pallet::storage]
    #[pallet::getter(fn something)]
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
    pub enum Error<T> {}

    // ---------------------------------------------------------------------------------------------
    //                                      Extrinsics
    // ---------------------------------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000)]
        pub fn create_amm(origin: OriginFor<T>, fees_bps: T::Balance) -> DispatchResult {
            ensure_root(origin)?;

            AmmCount::<T>::try_mutate(|count: &mut T::AmmId| -> Result<(), _> {
                let event = Event::<T>::AmmCreated(*count);
                AmmStates::<T>::insert(
                    *count,
                    Amm {
                        total_shares: Zero::zero(),
                        base_reserves: Zero::zero(),
                        quote_reserves: Zero::zero(),
                        fees_bps,
                    },
                );
                *count = count
                    .checked_add(&One::one())
                    .ok_or(ArithmeticError::Overflow)?;
                Self::deposit_event(event);
                Ok(())
            })
        }
    }
}
