use crate::{mock::*, Event};
use frame_support::{assert_noop, assert_ok, error::BadOrigin, pallet_prelude::Hooks};

pub const DEFAULT_BASE_ASSET: AssetId = 0;
pub const DEFAULT_QUOTE_ASSET: AssetId = 1;
pub const DEFAULT_FEES_BPS: Balance = 30;

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
fn first_liquidity_provider_sets_reserves() {
    todo!()
}
