use crate::{mock::*, Error, Event};
use frame_support::{
    assert_noop, assert_ok, error::BadOrigin, pallet_prelude::Hooks, traits::fungibles::Inspect,
};

// -------------------------------------------------------------------------------------------------
//                                          Setup
// -------------------------------------------------------------------------------------------------
pub const DEFAULT_BASE_ASSET: AssetId = 0;
pub const DEFAULT_QUOTE_ASSET: AssetId = 1;
pub const DEFAULT_FEES_BPS: Balance = 30;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const DOT: AssetId = 0;
pub const USDC: AssetId = 1;
pub const KSM: AssetId = 2;
pub const UNIT: Balance = 10_u64.pow(DEFAULT_DECIMALS as u32) as Balance;

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
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
            accounts: vec![
                (DOT, ALICE, UNIT),
                (USDC, ALICE, UNIT * 100),
                (KSM, ALICE, UNIT / 2),
                (DOT, BOB, UNIT * 100),
                (USDC, BOB, UNIT),
                (KSM, BOB, UNIT * 2),
            ],
        }
    }
}

pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            System::on_finalize(System::block_number());
        }
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
    }
}

#[test]
fn ext_builder_works() {
    ExtBuilder {
        accounts: vec![(USDC, ALICE, UNIT)],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        assert_eq!(<Assets as Inspect<AccountId>>::balance(USDC, &ALICE), UNIT);
    })
}
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------

#[test]
fn only_root_can_create_amm() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TestPallet::create_amm(
                Origin::signed(ALICE),
                DEFAULT_BASE_ASSET,
                DEFAULT_QUOTE_ASSET,
                DEFAULT_FEES_BPS
            ),
            BadOrigin
        );

        assert_ok!(TestPallet::create_amm(
            Origin::root(),
            DEFAULT_BASE_ASSET,
            DEFAULT_QUOTE_ASSET,
            DEFAULT_FEES_BPS
        ));
    });
}

#[test]
fn create_amm_increments_amm_counter() {
    new_test_ext().execute_with(|| {
        let before = TestPallet::amm_count();

        assert_ok!(TestPallet::create_amm(
            Origin::root(),
            DEFAULT_BASE_ASSET,
            DEFAULT_QUOTE_ASSET,
            DEFAULT_FEES_BPS
        ));

        assert_eq!(TestPallet::amm_count(), before + 1);
    })
}

#[test]
fn create_amm_emits_event() {
    new_test_ext().execute_with(|| {
        // For events to be registered
        run_to_block(1);

        assert_ok!(TestPallet::create_amm(
            Origin::root(),
            DEFAULT_BASE_ASSET,
            DEFAULT_QUOTE_ASSET,
            DEFAULT_FEES_BPS
        ));

        System::assert_last_event(Event::AmmCreated(0).into());
    })
}

#[test]
fn cant_provide_liquidity_to_nonexistent_amm() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            TestPallet::provide_liquidity(Origin::signed(ALICE), 0, UNIT, UNIT),
            Error::<Runtime>::InvalidAmmId,
        );
    });
}

#[test]
fn cant_provide_liquidity_if_holds_insufficient_token() {
    ExtBuilder {
        accounts: vec![(DOT, ALICE, UNIT), (USDC, ALICE, UNIT * 100)],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        assert_ok!(TestPallet::create_amm(
            Origin::root(),
            DOT,
            USDC,
            DEFAULT_FEES_BPS,
        ));

        assert_noop!(
            TestPallet::provide_liquidity(Origin::signed(ALICE), 0, UNIT * 2, UNIT * 200),
            pallet_assets::Error::<Runtime>::BalanceLow,
        );
    })
}
