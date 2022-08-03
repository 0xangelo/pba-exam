#![cfg(test)]

use crate as pallet_kitties;
use frame_support::{
    parameter_types,
    traits::{ConstU32, ConstU64, GenesisBuild}, PalletId,
};
use pallet_assets::FrozenBalance;
use pallet_kitties::Gender;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, Zero},
    BuildStorage, FixedU128,
};

// -------------------------------------------------------------------------------------------------
//                                          Runtime
// -------------------------------------------------------------------------------------------------

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

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
        Dex: pallet_dex,
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
        SubstrateKitties: pallet_kitties::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

// -------------------------------------------------------------------------------------------------
//                                          Types
// -------------------------------------------------------------------------------------------------

pub type AccountId = u64;
pub type AmmId = u64;
pub type AssetId = u32;
pub type Balance = u64;

// -------------------------------------------------------------------------------------------------
//                                          System
// -------------------------------------------------------------------------------------------------

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
//                                          Balances
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

// -------------------------------------------------------------------------------------------------
//                                          DEX
// -------------------------------------------------------------------------------------------------

parameter_types! {
    pub const TestPalletId: PalletId = PalletId(*b"test_pid");
    pub const DefaultDecimals: u8 = DEFAULT_DECIMALS;
}

impl pallet_dex::Config for Test {
    type AmmId = AmmId;
    type AssetId = AssetId;
    type Assets = Assets;
    type Balance = Balance;
    type Decimal = FixedU128;
    type DefaultDecimals = DefaultDecimals;
    type Event = Event;
    type PalletId = TestPalletId;
}

// -------------------------------------------------------------------------------------------------
//                                          Kitties
// -------------------------------------------------------------------------------------------------

parameter_types! {
    // One can owned at most 9,999 Kitties
    pub const MaxKittiesOwned: u32 = 9999;
}

impl pallet_kitties::Config for Test {
    type AssetId = AssetId;
    type Assets = Assets;
    type Balance = Balance;
    type Event = Event;
    type KittyRandomness = RandomnessCollectiveFlip;
    type MaxKittiesOwned = MaxKittiesOwned;
}

// -------------------------------------------------------------------------------------------------
//                                          Randomness
// -------------------------------------------------------------------------------------------------

impl pallet_randomness_collective_flip::Config for Test {}

// -------------------------------------------------------------------------------------------------
//                                          Testing Setup
// -------------------------------------------------------------------------------------------------

pub const DEFAULT_DECIMALS: u8 = 6;
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const DOT: AssetId = 0;
pub const USDC: AssetId = 1;
pub const KSM: AssetId = 2;
pub const UNIT: Balance = 10_u64.pow(DEFAULT_DECIMALS as u32) as Balance;

pub(crate) fn new_test_ext(
    users: Vec<(u64, [u8; 16], Gender)>,
    accounts: Vec<(AssetId, AccountId, Balance)>,
) -> sp_io::TestExternalities {
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

    pallet_assets::GenesisConfig::<Test> {
        accounts,
        // Hardcode assets and metadata temporarily
        assets: vec![(0, 0, true, 1), (1, 0, true, 1), (2, 0, true, 1)],
        metadata: vec![
            (
                DOT,
                (*b"Polkadot").into(),
                (*b"DOT").into(),
                DEFAULT_DECIMALS,
            ),
            (
                USDC,
                (*b"USD Coin").into(),
                (*b"USDC").into(),
                DEFAULT_DECIMALS,
            ),
            (KSM, (*b"Kusama").into(), (*b"KSM").into(), DEFAULT_DECIMALS),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
