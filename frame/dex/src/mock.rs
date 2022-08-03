use crate as pallet_dex;
use frame_support::{
    parameter_types,
    traits::{ConstU16, ConstU32, ConstU64, GenesisBuild},
    PalletId,
};
use frame_system as system;
use pallet_assets::FrozenBalance;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, Zero},
};

// -------------------------------------------------------------------------------------------------
//                                          Runtime
// -------------------------------------------------------------------------------------------------

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
pub type Block = frame_system::mocking::MockBlock<Runtime>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Assets: pallet_assets,
        Balances: pallet_balances,
        TestPallet: pallet_dex,
    }
);

// -------------------------------------------------------------------------------------------------
//                                          Types & Consts
// -------------------------------------------------------------------------------------------------

pub const DEFAULT_DECIMALS: u8 = 6;

pub type AccountId = u64;
pub type AmmId = u64;
pub type AssetId = u32;
pub type Balance = u64;

// -------------------------------------------------------------------------------------------------
//                                          System
// -------------------------------------------------------------------------------------------------

impl system::Config for Runtime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

// -------------------------------------------------------------------------------------------------
//                                          Assets
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

impl pallet_assets::Config for Runtime {
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
//                                          Balances
// -------------------------------------------------------------------------------------------------

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ConstU64<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

// -------------------------------------------------------------------------------------------------
//                                          DEX
// -------------------------------------------------------------------------------------------------

parameter_types! {
    pub const TestPalletId: PalletId = PalletId(*b"test_pid");
    pub const DefaultDecimals: u8 = DEFAULT_DECIMALS;
}

impl pallet_dex::Config for Runtime {
    type AmmId = AmmId;
    type AssetId = AssetId;
    type Assets = Assets;
    type Balance = Balance;
    type DefaultDecimals = DefaultDecimals;
    type Event = Event;
    type PalletId = TestPalletId;
}

// -------------------------------------------------------------------------------------------------
//                                          Testing Setup
// -------------------------------------------------------------------------------------------------

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap()
        .into()
}

pub struct ExtBuilder {
    /// Genesis assets: id, owner, is_sufficient, min_balance
    pub assets: Vec<(AssetId, AccountId, bool, Balance)>,
    /// Genesis metadata: id, name, symbol, decimals
    pub metadata: Vec<(AssetId, Vec<u8>, Vec<u8>, u8)>,
    /// Genesis accounts: id, account_id, balance
    pub accounts: Vec<(AssetId, AccountId, Balance)>,
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        pallet_assets::GenesisConfig::<Runtime> {
            assets: self.assets,
            metadata: self.metadata,
            accounts: self.accounts,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        storage.into()
    }
}
