#![cfg(test)]

use crate::{mock::*, pallet::Error, *};
use frame_support::{assert_noop, assert_ok};
use pallet_dex::{types::AssetType, traits::SimulateSwap};

// This function checks that kitty ownership is set correctly in storage.
// This will panic if things are not correct.
fn assert_ownership(owner: u64, kitty_id: [u8; 16]) {
    // For a kitty to be owned it should exist.
    let kitty = Kitties::<Test>::get(kitty_id).unwrap();
    // The kitty's owner is set correctly.
    assert_eq!(kitty.owner, owner);

    for (check_owner, owned) in KittiesOwned::<Test>::iter() {
        if owner == check_owner {
            // Owner should have this kitty.
            assert!(owned.contains(&kitty_id));
        } else {
            // Everyone else should not.
            assert!(!owned.contains(&kitty_id));
        }
    }
}

#[test]
fn should_build_genesis_kitties() {
    new_test_ext(
        vec![
            (1, *b"1234567890123456", Gender::Female),
            (2, *b"123456789012345a", Gender::Male),
        ],
        vec![],
    )
    .execute_with(|| {
        // Check we have 2 kitties, as specified in genesis
        assert_eq!(CountForKitties::<Test>::get(), 2);

        // Check owners own the correct amount of kitties
        let kitties_owned_by_1 = KittiesOwned::<Test>::get(1);
        assert_eq!(kitties_owned_by_1.len(), 1);

        let kitties_owned_by_2 = KittiesOwned::<Test>::get(2);
        assert_eq!(kitties_owned_by_2.len(), 1);

        // Check that kitties are owned by the correct owners
        let kitty_1 = kitties_owned_by_1[0];
        assert_ownership(1, kitty_1);

        let kitty_2 = kitties_owned_by_2[0];
        assert_ownership(2, kitty_2);
    });
}

#[test]
fn create_kitty_should_work() {
    new_test_ext(vec![], vec![]).execute_with(|| {
        // Create a kitty with account #10
        assert_ok!(SubstrateKitties::create_kitty(Origin::signed(10)));

        // Check that now 3 kitties exists
        assert_eq!(CountForKitties::<Test>::get(), 1);

        // Check that account #10 owns 1 kitty
        let kitties_owned = KittiesOwned::<Test>::get(10);
        assert_eq!(kitties_owned.len(), 1);
        let id = kitties_owned.last().unwrap();
        assert_ownership(10, *id);

        // Check that multiple create_kitty calls work in the same block.
        // Increment extrinsic index to add entropy for DNA
        frame_system::Pallet::<Test>::set_extrinsic_index(1);
        assert_ok!(SubstrateKitties::create_kitty(Origin::signed(10)));
    });
}

#[test]
fn create_kitty_fails() {
    // Check that create_kitty fails when user owns too many kitties.
    new_test_ext(vec![], vec![]).execute_with(|| {
        // Create `MaxKittiesOwned` kitties with account #10
        for _i in 0..<Test as Config>::MaxKittiesOwned::get() {
            assert_ok!(SubstrateKitties::create_kitty(Origin::signed(10)));
            // We do this because the hash of the kitty depends on this for seed,
            // so changing this allows you to have a different kitty id
            System::set_block_number(System::block_number() + 1);
        }

        // Can't create 1 more
        assert_noop!(
            SubstrateKitties::create_kitty(Origin::signed(10)),
            Error::<Test>::TooManyOwned
        );

        // Minting a kitty with DNA that already exists should fail
        let id = [0u8; 16];

        // Mint new kitty with `id`
        assert_ok!(SubstrateKitties::mint(&1, id, Gender::Male));

        // Mint another kitty with the same `id` should fail
        assert_noop!(
            SubstrateKitties::mint(&1, id, Gender::Male),
            Error::<Test>::DuplicateKitty
        );
    });
}

#[test]
fn transfer_kitty_should_work() {
    new_test_ext(vec![], vec![]).execute_with(|| {
        // Account 10 creates a kitty
        assert_ok!(SubstrateKitties::create_kitty(Origin::signed(10)));
        let id = KittiesOwned::<Test>::get(10)[0];

        // and sends it to account 3
        assert_ok!(SubstrateKitties::transfer(Origin::signed(10), 3, id));

        // Check that account 10 now has nothing
        assert_eq!(KittiesOwned::<Test>::get(10).len(), 0);

        // but account 3 does
        assert_eq!(KittiesOwned::<Test>::get(3).len(), 1);
        assert_ownership(3, id);
    });
}

#[test]
fn transfer_kitty_should_fail() {
    new_test_ext(
        vec![
            (1, *b"1234567890123456", Gender::Female),
            (2, *b"123456789012345a", Gender::Male),
        ],
        vec![],
    )
    .execute_with(|| {
        // Get the DNA of some kitty
        let dna = KittiesOwned::<Test>::get(1)[0];

        // Account 9 cannot transfer a kitty with this DNA.
        assert_noop!(
            SubstrateKitties::transfer(Origin::signed(9), 2, dna),
            Error::<Test>::NotOwner
        );

        // Check transfer fails when transferring to self
        assert_noop!(
            SubstrateKitties::transfer(Origin::signed(1), 1, dna),
            Error::<Test>::TransferToSelf
        );

        // Check transfer fails when no kitty exists
        let random_id = [0u8; 16];

        assert_noop!(
            SubstrateKitties::transfer(Origin::signed(2), 1, random_id),
            Error::<Test>::NoKitty
        );

        // Check that transfer fails when max kitty is reached
        // Create `MaxKittiesOwned` kitties for account #10
        for _i in 0..<Test as Config>::MaxKittiesOwned::get() {
            assert_ok!(SubstrateKitties::create_kitty(Origin::signed(10)));
            System::set_block_number(System::block_number() + 1);
        }

        // Account #10 should not be able to receive a new kitty
        assert_noop!(
            SubstrateKitties::transfer(Origin::signed(1), 10, dna),
            Error::<Test>::TooManyOwned
        );
    });
}

#[test]
fn breed_kitty_works() {
    // Check that breed kitty works as expected.
    new_test_ext(vec![(2, *b"123456789012345a", Gender::Male)], vec![]).execute_with(|| {
        // Get mom and dad kitties from account #1
        let mom = [0u8; 16];
        assert_ok!(SubstrateKitties::mint(&1, mom, Gender::Female));

        // Mint male kitty for account #1
        let dad = [1u8; 16];
        assert_ok!(SubstrateKitties::mint(&1, dad, Gender::Male));

        // Breeder can only breed kitties they own
        assert_ok!(SubstrateKitties::breed_kitty(Origin::signed(1), mom, dad));

        // Check the new kitty exists and DNA is from the mom and dad
        let new_dna = KittiesOwned::<Test>::get(1)[2];
        for &i in new_dna.iter() {
            assert!(i == 0u8 || i == 1u8)
        }

        // Kitty cant breed with itself
        assert_noop!(
            SubstrateKitties::breed_kitty(Origin::signed(1), mom, mom),
            Error::<Test>::CantBreed
        );

        // Two kitties must be bred by the same owner
        // Get the kitty owned by account #1
        let kitty_1 = KittiesOwned::<Test>::get(1)[0];

        // Another kitty from another owner
        let kitty_2 = KittiesOwned::<Test>::get(2)[0];

        // Breeder can only breed kitties they own
        assert_noop!(
            SubstrateKitties::breed_kitty(Origin::signed(1), kitty_1, kitty_2),
            Error::<Test>::NotOwner
        );
    });
}

#[test]
fn breed_kitty_fails() {
    new_test_ext(vec![], vec![]).execute_with(|| {
        // Check that breed_kitty checks opposite gender
        let kitty_1 = [1u8; 16];
        let kitty_2 = [3u8; 16];

        // Mint two Female kitties
        assert_ok!(SubstrateKitties::mint(&3, kitty_1, Gender::Female));
        assert_ok!(SubstrateKitties::mint(&3, kitty_2, Gender::Female));

        // And a male kitty
        let kitty_3 = [4u8; 16];
        assert_ok!(SubstrateKitties::mint(&3, kitty_3, Gender::Male));

        // Same gender kitty can't breed
        assert_noop!(
            SubstrateKitties::breed_kitty(Origin::signed(3), kitty_1, kitty_2),
            Error::<Test>::CantBreed
        );

        // Check that breed kitty fails with too many kitties
        // Account 3 already has 3 kitties so we subtract that from our max
        for _i in 0..<Test as Config>::MaxKittiesOwned::get() - 3 {
            assert_ok!(SubstrateKitties::create_kitty(Origin::signed(3)));
            // We do this to avoid getting a `DuplicateKitty` error
            System::set_block_number(System::block_number() + 1);
        }

        // Breed should fail if breeder has reached MaxKittiesOwned
        assert_noop!(
            SubstrateKitties::breed_kitty(Origin::signed(3), kitty_1, kitty_3),
            Error::<Test>::TooManyOwned
        );
    });
}

#[test]
fn dna_helpers_work_as_expected() {
    new_test_ext(vec![], vec![]).execute_with(|| {
        // Test gen_dna and other dna functions behave as expected
        // Get two kitty dnas
        let dna_1 = [1u8; 16];
        let dna_2 = [2u8; 16];

        // Generate unique Gender and DNA
        let (dna, _) = SubstrateKitties::breed_dna(&dna_1, &dna_2);

        // Check that the new kitty is actually a child of one of its parents
        // DNA bytes must be a mix of mom or dad's DNA
        for &i in dna.iter() {
            assert!(i == 1u8 || i == 2u8)
        }

        // Test that randomness works in same block
        let (random_dna_1, _) = SubstrateKitties::gen_dna();
        // increment extrinsic index
        frame_system::Pallet::<Test>::set_extrinsic_index(1);
        let (random_dna_2, _) = SubstrateKitties::gen_dna();

        // Random values should not be equal
        assert_ne!(random_dna_1, random_dna_2);
    });
}

#[test]
fn buy_kitty_works() {
    new_test_ext(
        vec![
            (ALICE, *b"1234567890123456", Gender::Female),
            (BOB, *b"123456789012345a", Gender::Male),
        ],
        vec![(DOT, ALICE, UNIT), (DOT, CHARLIE, UNIT * 100)],
    )
    .execute_with(|| {
        // Check buy_kitty works as expected
        let id = KittiesOwned::<Test>::get(BOB)[0];
        let set_price = 4;
        let balance_alice_before = Assets::balance(DOT, &ALICE);
        let balance_bob_before = Assets::balance(DOT, &BOB);

        // Bob sets a price of 4 for their kitty
        assert_ok!(SubstrateKitties::set_price(
            Origin::signed(BOB),
            id,
            Some((set_price, DOT)),
        ));

        // Alice can buy Bob's kitty, specifying some limit_price
        let limit_price = 6;
        assert_ok!(SubstrateKitties::buy_kitty(
            Origin::signed(ALICE),
            id,
            limit_price
        ));

        // Check balance transfer works as expected
        let balance_alice_after = Assets::balance(DOT, &ALICE);
        let balance_bob_after = Assets::balance(DOT, &BOB);

        // We use set_price as this is the amount actually being charged
        assert_eq!(balance_alice_before - set_price, balance_alice_after);
        assert_eq!(balance_bob_before + set_price, balance_bob_after);

        // Now this kitty is not for sale, even from an account who can afford it
        assert_noop!(
            SubstrateKitties::buy_kitty(Origin::signed(CHARLIE), id, set_price),
            Error::<Test>::NotForSale
        );
    });
}

#[test]
fn buy_kitty_fails() {
    new_test_ext(
        vec![
            (ALICE, *b"1234567890123456", Gender::Female),
            (BOB, *b"123456789012345a", Gender::Male),
            (CHARLIE, *b"1234567890123410", Gender::Male),
        ],
        vec![
            (DOT, ALICE, UNIT),
            (DOT, BOB, UNIT * 10),
            (DOT, CHARLIE, UNIT / 2),
        ],
    )
    .execute_with(|| {
        // Check buy_kitty fails when kitty is not for sale
        let id = KittiesOwned::<Test>::get(ALICE)[0];
        // Kitty is not for sale
        assert_noop!(
            SubstrateKitties::buy_kitty(Origin::signed(BOB), id, 2),
            Error::<Test>::NotForSale
        );

        // Check buy_kitty fails when bid price is too low
        // New price is set to 4
        let id = KittiesOwned::<Test>::get(BOB)[0];
        let set_price = 4;
        assert_ok!(SubstrateKitties::set_price(
            Origin::signed(BOB),
            id,
            Some((set_price, DOT)),
        ));

        // Charlie can't buy this kitty for half the asking price
        assert_noop!(
            SubstrateKitties::buy_kitty(Origin::signed(CHARLIE), id, set_price / 2),
            Error::<Test>::BidPriceTooLow
        );

        // Check buy_kitty fails when balance is too low
        // Get the balance of account 10
        let balance_of_charlie = Assets::balance(DOT, &CHARLIE);

        // Reset the price to something higher than Charlie's balance
        assert_ok!(SubstrateKitties::set_price(
            Origin::signed(BOB),
            id,
            Some((balance_of_charlie * 10, DOT)),
        ));

        // Account 10 can't buy a kitty they can't afford
        assert_noop!(
            SubstrateKitties::buy_kitty(Origin::signed(CHARLIE), id, balance_of_charlie * 10),
            pallet_assets::Error::<Test>::BalanceLow
        );
    });
}

#[test]
fn set_price_works() {
    new_test_ext(
        vec![
            (1, *b"1234567890123456", Gender::Female),
            (2, *b"123456789012345a", Gender::Male),
        ],
        vec![(DOT, ALICE, UNIT), (DOT, BOB, UNIT * 10)],
    )
    .execute_with(|| {
        // Check set_price works as expected
        let id = KittiesOwned::<Test>::get(2)[0];
        let set_price = 4;
        assert_ok!(SubstrateKitties::set_price(
            Origin::signed(2),
            id,
            Some((set_price, DOT)),
        ));

        // Only owner can set price
        assert_noop!(
            SubstrateKitties::set_price(Origin::signed(1), id, Some((set_price, DOT))),
            Error::<Test>::NotOwner
        );

        // Kitty must exist too
        let non_dna = [2u8; 16];
        assert_noop!(
            SubstrateKitties::set_price(Origin::signed(1), non_dna, Some((set_price, DOT))),
            Error::<Test>::NoKitty
        );
    });
}

// -------------------------------------------------------------------------------------------------
//                                      'Integration' tests
// -------------------------------------------------------------------------------------------------

pub const DEFAULT_SHARE_ASSET: AssetId = 100;

#[test]
fn can_use_dex_to_obtain_token_for_kitty_purchase() {
    new_test_ext(
        vec![
            (ALICE, *b"1234567890123456", Gender::Female),
        ],
        vec![
            (DOT, BOB, UNIT),
            (DOT, CHARLIE, UNIT * 5),
            (USDC, CHARLIE, UNIT * 500)
        ],
    )
    .execute_with(|| {
        assert_ok!(Dex::create_amm(
            Origin::signed(CHARLIE),
            DOT,
            USDC,
            DEFAULT_SHARE_ASSET,
            30, // 30 bps, or 0.3%
        ));

        // Charlie initializes the AMM by providing liquidity
        assert_ok!(Dex::provide_liquidity(
            Origin::signed(CHARLIE),
            0,
            UNIT * 5,
            UNIT * 500,
        ));

        // Alice sets a price of 40 USDC for her kitty
        let id = KittiesOwned::<Test>::get(ALICE)[0];
        let set_price = 40 * UNIT;
        assert_ok!(SubstrateKitties::set_price(
            Origin::signed(ALICE),
            id,
            Some((set_price, USDC)),
        ));

        // Bob queries the DEX for how much DOT he would need to swap to get the required USDC to
        // buy Alice's kitty.
        let dot_required = <Dex as SimulateSwap>::output_price(
            0,
            AssetType::Quote,
            40 * UNIT
        ).unwrap();

        // Bob uses the DOT amount computed to aquire the USDC needed.
        assert_ok!(Dex::swap(
            Origin::signed(BOB),
            0,
            AssetType::Base,
            dot_required * 1001 / 1000, // Send an extra bit to account for rounding errors
            0
        ));

        // Bob can buy Alice's kitty, specifying some limit_price
        assert_ok!(SubstrateKitties::buy_kitty(
            Origin::signed(BOB),
            id,
            40 * UNIT
        ));

    });
}
