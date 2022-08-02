#![cfg(test)]

use crate as pallet_kitties;
use frame_support::{parameter_types, traits::{ConstU32, ConstU64}};
use pallet_assets::FrozenBalance;
use pallet_kitties::Gender;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, Zero},
    BuildStorage,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type AccountId = u64;
pub type AssetId = u32;
pub type Balance = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Assets: pallet_assets,
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
        SubstrateKitties: pallet_kitties::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type AccountData = pallet_balances::AccountData<u64>;
    type AccountId = AccountId;
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockHashCount = BlockHashCount;
    type BlockLength = ();
    type BlockNumber = u64;
    type BlockWeights = ();
    type Call = Call;
    type DbWeight = ();
    type Event = Event;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type Index = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type OnKilledAccount = ();
    type OnNewAccount = ();
    type OnSetCode = ();
    type Origin = Origin;
    type PalletInfo = PalletInfo;
    type SS58Prefix = SS58Prefix;
    type SystemWeightInfo = ();
    type Version = ();
}

// -------------------------------------------------------------------------------------------------
// Copied from https://github.com/paritytech/substrate/blob/master/frame/assets/src/mock.rs
// -------------------------------------------------------------------------------------------------
use std::{cell::RefCell, collections::HashMap};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum Hook {
    Died(u32, u64),
}
thread_local! {
    static FROZEN: RefCell<HashMap<(u32, u64), u64>> = RefCell::new(Default::default());
    static HOOKS: RefCell<Vec<Hook>> = RefCell::new(Default::default());
}

pub struct TestFreezer;
impl FrozenBalance<u32, u64, u64> for TestFreezer {
    fn frozen_balance(asset: u32, who: &u64) -> Option<u64> {
        FROZEN.with(|f| f.borrow().get(&(asset, *who)).cloned())
    }

    fn died(asset: u32, who: &u64) {
        HOOKS.with(|h| h.borrow_mut().push(Hook::Died(asset, *who)));
        // Sanity check: dead accounts have no balance.
        assert!(Assets::balance(asset, *who).is_zero());
    }
}

impl pallet_assets::Config for Test {
    type Event = Event;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetDeposit = ConstU64<1>;
    type AssetAccountDeposit = ConstU64<10>;
    type MetadataDepositBase = ConstU64<1>;
    type MetadataDepositPerByte = ConstU64<1>;
    type ApprovalDeposit = ConstU64<1>;
    type StringLimit = ConstU32<50>;
    type Freezer = TestFreezer;
    type Extra = ();
    type WeightInfo = ();
}
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type AccountStore = System;
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type WeightInfo = ();
}

parameter_types! {
    // One can owned at most 9,999 Kitties
    pub const MaxKittiesOwned: u32 = 9999;
}

impl pallet_kitties::Config for Test {
    type AssetId = AssetId;
    type Assets = Assets;
    type Balance = Balance;
    type Currency = Balances;
    type Event = Event;
    type KittyRandomness = RandomnessCollectiveFlip;
    type MaxKittiesOwned = MaxKittiesOwned;
}

impl pallet_randomness_collective_flip::Config for Test {}

pub(crate) fn new_test_ext(users: Vec<(u64, [u8; 16], Gender)>) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    GenesisConfig {
        //
        balances: BalancesConfig {
            balances: users.iter().map(|(user, _, _)| (*user, 10)).collect(),
        },
        substrate_kitties: SubstrateKittiesConfig {
            kitties: users
                .iter()
                .map(|(user, kitty, gender)| (*user, *kitty, *gender))
                .collect(),
        },
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
