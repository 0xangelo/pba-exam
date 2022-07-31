use crate::{mock::*, Event};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

pub const DEFAULT_FEES_BPS: Balance = 30;

#[test]
fn only_root_can_create_amm() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TestPallet::create_amm(Origin::signed(ALICE), DEFAULT_FEES_BPS),
            BadOrigin
        );

        assert_ok!(TestPallet::create_amm(Origin::root(), DEFAULT_FEES_BPS));
    });
}

#[test]
fn create_amm_increments_amm_counter() {
    new_test_ext().execute_with(|| {
        let before = TestPallet::amm_count();

        assert_ok!(TestPallet::create_amm(Origin::root(), DEFAULT_FEES_BPS));

        assert_eq!(TestPallet::amm_count(), before + 1);
    })
}

#[test]
fn create_amm_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(TestPallet::create_amm(Origin::root(), DEFAULT_FEES_BPS));

        System::assert_last_event(Event::AmmCreated(0).into());
    })
}
