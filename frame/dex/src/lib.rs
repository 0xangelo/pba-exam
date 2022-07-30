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

	// ---------------------------------------------------------------------------------------------
	//                                      Config
	// ---------------------------------------------------------------------------------------------

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Unique identifier for an AMM instance.
        type AmmId: Decode + Encode + FullCodec + MaxEncodedLen + TypeInfo;
        /// Type of balances for user accounts and AMM reserves.
        type Balance: Decode + Encode + MaxEncodedLen + TypeInfo;
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

    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    #[codec(mel_bound())]
    pub struct Amm<T: Config> {
        pub total_shares: T::Balance,
    }

    #[pallet::storage]
    #[pallet::getter(fn something)]
    pub type AmmStates<T: Config> = StorageMap<_, Blake2_128Concat, T::AmmId, Amm<T>>;

	// ---------------------------------------------------------------------------------------------
	//                                      Events
	// ---------------------------------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}

	// ---------------------------------------------------------------------------------------------
	//                                      Errors
	// ---------------------------------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {}

	// ---------------------------------------------------------------------------------------------
	//                                      Extrinsics
	// ---------------------------------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}
