use crate::{mock::*, types::AssetType, Error, Event};
use frame_support::{
    assert_noop, assert_ok,
    error::BadOrigin,
    pallet_prelude::Hooks,
    traits::fungibles::{Create, Inspect},
};
use pallet_assets::Error as AssetsError;

// -------------------------------------------------------------------------------------------------
//                                          Setup
// -------------------------------------------------------------------------------------------------
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const DOT: AssetId = 0;
pub const USDC: AssetId = 1;
pub const KSM: AssetId = 2;
pub const UNIT: Balance = 10_u64.pow(DEFAULT_DECIMALS as u32) as Balance;

pub const DEFAULT_BASE_ASSET: AssetId = DOT;
pub const DEFAULT_QUOTE_ASSET: AssetId = USDC;
pub const DEFAULT_SHARE_ASSET: AssetId = 100;
pub const DEFAULT_FEES_BPS: Balance = 30;

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            assets: vec![(DOT, 0, true, 1), (USDC, 0, true, 1), (KSM, 0, true, 1)],
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

// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------

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

#[test]
fn only_root_can_create_amm() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TestPallet::create_amm(
                Origin::signed(ALICE),
                DEFAULT_BASE_ASSET,
                DEFAULT_QUOTE_ASSET,
                DEFAULT_SHARE_ASSET,
                DEFAULT_FEES_BPS
            ),
            BadOrigin
        );

        assert_ok!(TestPallet::create_amm(
            Origin::root(),
            DEFAULT_BASE_ASSET,
            DEFAULT_QUOTE_ASSET,
            DEFAULT_SHARE_ASSET,
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
            DEFAULT_SHARE_ASSET,
            DEFAULT_FEES_BPS
        ));

        assert_eq!(TestPallet::amm_count(), before + 1);
    })
}

#[test]
fn creating_the_same_asset_fails() {
    new_test_ext().execute_with(|| {
        <Assets as Create<AccountId>>::create(DEFAULT_SHARE_ASSET, 0, true, 1).unwrap();
        assert_noop!(
            <Assets as Create<AccountId>>::create(DEFAULT_SHARE_ASSET, 0, true, 1),
            AssetsError::<Runtime>::InUse
        );
    })
}

#[test]
fn cant_create_amm_with_existing_lp_asset() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(<Assets as Create<AccountId>>::create(
            DEFAULT_SHARE_ASSET,
            0,
            true,
            1
        ));

        assert_noop!(
            TestPallet::create_amm(
                Origin::root(),
                DEFAULT_BASE_ASSET,
                DEFAULT_QUOTE_ASSET,
                DEFAULT_SHARE_ASSET,
                DEFAULT_FEES_BPS
            ),
            Error::<Runtime>::InvalidShareAsset
        );
    })
}

// #[test]
// fn create_amm_creates_new_liquidity_provider_asset() {
//     ExtBuilder::default().build().execute_with(|| {
//         // Runtime starts with no LP asset registered
//         assert_eq!(
//             <Assets as InspectMetadata<AccountId>>::name(&DEFAULT_SHARE_ASSET),
//             Vec::<u8>::new()
//         );

//         assert_ok!(TestPallet::create_amm(
//             Origin::root(),
//             DOT,
//             USDC,
//             DEFAULT_SHARE_ASSET,
//             DEFAULT_FEES_BPS
//         ));

//         let expected: Vec<u8> = b"DOT-USDC-LP"[..].into();
//         assert_eq!(
//             <Assets as InspectMetadata<AccountId>>::name(&DEFAULT_SHARE_ASSET),
//             expected
//         )
//     })
// }

fn default_amm() {
    assert_ok!(TestPallet::create_amm(
        Origin::root(),
        DOT,
        USDC,
        DEFAULT_SHARE_ASSET,
        DEFAULT_FEES_BPS,
    ));
}

#[test]
fn create_amm_emits_event() {
    new_test_ext().execute_with(|| {
        // For events to be registered
        run_to_block(1);

        default_amm();

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
        default_amm();

        assert_noop!(
            TestPallet::provide_liquidity(Origin::signed(ALICE), 0, UNIT * 2, UNIT * 200),
            AssetsError::<Runtime>::BalanceLow,
        );
    })
}

#[test]
fn first_liquidity_provider_initializes_amm() {
    ExtBuilder {
        accounts: vec![(DOT, ALICE, UNIT), (USDC, ALICE, UNIT * 100)],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        default_amm();

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(ALICE),
            0,
            UNIT,
            UNIT * 100,
        ));

        let amm_state = TestPallet::amm_state(0).unwrap();
        assert_eq!(amm_state.base_reserves, UNIT);
        assert_eq!(amm_state.quote_reserves, UNIT * 100);

        assert_eq!(
            <Assets as Inspect<AccountId>>::balance(DEFAULT_SHARE_ASSET, &ALICE),
            amm_state.total_shares
        );
        assert_eq!(amm_state.total_shares, 100 * UNIT);

        System::assert_last_event(
            Event::LiquidityAdded {
                amm_id: 0,
                user: ALICE,
                shares: 100 * UNIT,
            }
            .into(),
        );
    })
}

#[test]
fn second_liquidity_provider_adds_to_the_pool() {
    ExtBuilder {
        accounts: vec![
            (DOT, ALICE, UNIT),
            (USDC, ALICE, UNIT * 100),
            (DOT, BOB, UNIT),
            (USDC, BOB, UNIT * 100),
        ],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        default_amm();

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(ALICE),
            0,
            UNIT,
            UNIT * 100,
        ));

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(BOB),
            0,
            UNIT / 2,
            UNIT * 50,
        ));

        let amm_state = TestPallet::amm_state(0).unwrap();
        assert_eq!(amm_state.base_reserves, UNIT + UNIT / 2);
        assert_eq!(amm_state.quote_reserves, 150 * UNIT);
        assert_eq!(amm_state.total_shares, 150 * UNIT);
    })
}

#[test]
fn cannot_withdraw_from_nonexistent_amm() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TestPallet::withdraw(Origin::signed(ALICE), 0, UNIT),
            Error::<Runtime>::InvalidAmmId
        );
    })
}

#[test]
fn cannot_withdraw_if_doesnt_have_shares() {
    ExtBuilder::default().build().execute_with(|| {
        default_amm();

        assert_noop!(
            TestPallet::withdraw(Origin::signed(ALICE), 0, UNIT),
            Error::<Runtime>::InvalidShareAmount
        );
    })
}

#[test]
fn withdraw_returns_share_of_pool_assets() {
    ExtBuilder {
        accounts: vec![
            (DOT, ALICE, UNIT),
            (USDC, ALICE, UNIT * 100),
            (DOT, BOB, UNIT / 2),
            (USDC, BOB, UNIT * 50),
        ],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        default_amm();

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(ALICE),
            0,
            UNIT,
            UNIT * 100,
        ));

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(BOB),
            0,
            UNIT / 2,
            UNIT * 50,
        ));

        assert_ok!(TestPallet::withdraw(Origin::signed(ALICE), 0, 100 * UNIT));
        assert_eq!(
            <Assets as Inspect<AccountId>>::balance(DEFAULT_SHARE_ASSET, &ALICE),
            0
        );
        assert_eq!(<Assets as Inspect<AccountId>>::balance(DOT, &ALICE), UNIT);
        assert_eq!(
            <Assets as Inspect<AccountId>>::balance(USDC, &ALICE),
            UNIT * 100
        );
        System::assert_last_event(
            Event::LiquidityRemoved {
                amm_id: 0,
                user: ALICE,
                shares: 100 * UNIT,
            }
            .into(),
        );

        assert_ok!(TestPallet::withdraw(Origin::signed(BOB), 0, 50 * UNIT));
        assert_eq!(
            <Assets as Inspect<AccountId>>::balance(DEFAULT_SHARE_ASSET, &BOB),
            0
        );
        assert_eq!(<Assets as Inspect<AccountId>>::balance(DOT, &BOB), UNIT / 2);
        assert_eq!(
            <Assets as Inspect<AccountId>>::balance(USDC, &BOB),
            UNIT * 50
        );
        System::assert_last_event(
            Event::LiquidityRemoved {
                amm_id: 0,
                user: BOB,
                shares: 50 * UNIT,
            }
            .into(),
        );

        let amm_state = TestPallet::amm_state(0).unwrap();
        assert_eq!(amm_state.total_shares, 0);
        assert_eq!(amm_state.base_reserves, 0);
        assert_eq!(amm_state.quote_reserves, 0);
    })
}

#[test]
fn should_not_swap_against_inexistent_amm() {
    ExtBuilder {
        accounts: vec![(DOT, ALICE, UNIT), (USDC, ALICE, UNIT * 100)],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        assert_noop!(
            TestPallet::swap(Origin::signed(ALICE), 0, AssetType::Quote, UNIT, UNIT / 100,),
            Error::<Runtime>::InvalidAmmId
        );
    })
}

#[test]
fn should_not_swap_against_unitialized_amm() {
    ExtBuilder {
        accounts: vec![(DOT, ALICE, UNIT), (USDC, ALICE, UNIT * 100)],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        default_amm();

        assert_noop!(
            TestPallet::swap(Origin::signed(ALICE), 0, AssetType::Quote, UNIT, UNIT / 100,),
            Error::<Runtime>::ZeroLiquidity
        );
    })
}

#[test]
fn swap_quote_returns_base_asset() {
    ExtBuilder {
        accounts: vec![
            (DOT, ALICE, UNIT),
            (USDC, ALICE, UNIT * 100),
            (DOT, BOB, UNIT),
            (USDC, BOB, UNIT * 100),
        ],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        default_amm();

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(ALICE),
            0,
            UNIT,
            UNIT * 100,
        ));

        assert_ok!(TestPallet::swap(
            Origin::signed(BOB),
            0,
            AssetType::Quote,
            UNIT,
            UNIT / 102,
        ));

        assert!(
            <Assets as Inspect<AccountId>>::balance(DEFAULT_BASE_ASSET, &BOB) < UNIT + UNIT / 100
        );

        // TODO: assert last event matches Event::<T>::Swapped but without computing exact expected amounts
    })
}

#[test]
fn swap_quote_incurs_slippage() {
    ExtBuilder {
        accounts: vec![
            (DOT, ALICE, UNIT),
            (USDC, ALICE, UNIT * 100),
            (DOT, BOB, UNIT),
            (USDC, BOB, UNIT * 100),
        ],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        // No fees, just testing slippage due to x * y = K now.
        assert_ok!(TestPallet::create_amm(
            Origin::root(),
            DOT,
            USDC,
            DEFAULT_SHARE_ASSET,
            0,
        ));

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(ALICE),
            0,
            UNIT,
            UNIT * 100,
        ));

        // Current price is 100 USDC / DOT, but due to slippage BOB will get less than that amount.
        assert_noop!(
            TestPallet::swap(Origin::signed(BOB), 0, AssetType::Quote, UNIT, UNIT / 100,),
            Error::<Runtime>::SlippageExceeded
        );
    })
}

#[test]
fn swap_base_returns_quote_asset() {
    ExtBuilder {
        accounts: vec![
            (DOT, ALICE, UNIT),
            (USDC, ALICE, UNIT * 100),
            (DOT, BOB, UNIT),
            (USDC, BOB, UNIT * 100),
        ],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        default_amm();

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(ALICE),
            0,
            UNIT,
            UNIT * 100,
        ));

        // quote - (k / (base + .5)) ~= 33.3333
        assert_ok!(TestPallet::swap(
            Origin::signed(BOB),
            0,
            AssetType::Base,
            UNIT / 2,
            UNIT * 32
        ));

        assert!(<Assets as Inspect<AccountId>>::balance(DEFAULT_QUOTE_ASSET, &BOB) < UNIT * 150);

        // TODO: assert last event matches Event::<T>::Swapped but without computing exact expected amounts
    })
}

#[test]
fn swap_base_incurs_slippage() {
    ExtBuilder {
        accounts: vec![
            (DOT, ALICE, UNIT),
            (USDC, ALICE, UNIT * 100),
            (DOT, BOB, UNIT),
            (USDC, BOB, UNIT * 100),
        ],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        // No fees, just testing slippage due to x * y = K now.
        assert_ok!(TestPallet::create_amm(
            Origin::root(),
            DOT,
            USDC,
            DEFAULT_SHARE_ASSET,
            0,
        ));

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(ALICE),
            0,
            UNIT,
            UNIT * 100,
        ));

        // Current price is 100 USDC / DOT, but due to slippage BOB will get less than expected
        assert_noop!(
            TestPallet::swap(Origin::signed(BOB), 0, AssetType::Base, UNIT / 2, UNIT * 50),
            Error::<Runtime>::SlippageExceeded
        );
    })
}

#[test]
fn should_accrue_rewards_to_liquidity_providers() {
    ExtBuilder {
        accounts: vec![
            (DOT, ALICE, UNIT),
            (USDC, ALICE, UNIT * 100),
            (DOT, BOB, UNIT / 2),
            (USDC, BOB, UNIT * 50),
            (USDC, CHARLIE, UNIT * 10),
        ],
        ..Default::default()
    }
    .build()
    .execute_with(|| {
        run_to_block(1);

        default_amm();

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(ALICE),
            0,
            UNIT,
            UNIT * 100,
        ));

        assert_ok!(TestPallet::provide_liquidity(
            Origin::signed(BOB),
            0,
            UNIT / 2,
            UNIT * 50,
        ));

        // Charlie swaps twice and returns AMM back to initial reserve proportions.
        assert_ok!(TestPallet::swap(
            Origin::signed(CHARLIE),
            0,
            AssetType::Quote,
            UNIT * 10,
            0
        ));
        assert_ok!(TestPallet::swap(
            Origin::signed(CHARLIE),
            0,
            AssetType::Base,
            <Assets as Inspect<AccountId>>::balance(DOT, &CHARLIE),
            0
        ));
        // Ensure fees were charged during the two operations.
        assert_eq!(<Assets as Inspect<AccountId>>::balance(DOT, &CHARLIE), 0);
        assert!(<Assets as Inspect<AccountId>>::balance(USDC, &CHARLIE) < UNIT * 10);

        let amm_state = TestPallet::amm_state(0).unwrap();
        assert_eq!(amm_state.base_reserves, UNIT + UNIT / 2);
        assert!(amm_state.quote_reserves > UNIT * 150);

        // Bob withdraws his shares and realizes his rewards
        assert_ok!(TestPallet::withdraw(Origin::signed(BOB), 0, 50 * UNIT));
        assert_eq!(<Assets as Inspect<AccountId>>::balance(DOT, &BOB), UNIT / 2);
        assert!(<Assets as Inspect<AccountId>>::balance(USDC, &BOB) > UNIT * 50);
    })
}
