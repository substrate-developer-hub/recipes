//! A Simple Charity which holds and governs a pot of funds.
//!
//! The Charity has a pot of funds. The Pot is unique because unlike other token-holding accounts,
//! it is not controlled by a cryptographic keypair. Rather it belongs to the pallet itself.
//! Funds can be added to the pot in two ways:
//! * Anyone can make a donation through the `donate` extrinsic.
//! * An imablance can be absorbed from somewhere else in the runtime.
//! Funds can only be allocated by a root call to the `allocate` extrinsic/
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_runtime::{
    traits::{AccountIdConversion},
    ModuleId,
};
#[cfg(feature = "std")]
use frame_support::traits::{Currency, ExistenceRequirement::AllowDeath, OnUnbalanced, Imbalance};
use frame_support::{
	decl_event,
	decl_module,
	decl_storage,
	dispatch::{DispatchResult}
};
use frame_system::{self as system, ensure_signed, ensure_root};

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
type NegativeImbalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

/// Hardcoded pallet ID; used to create the special Pot Account
/// Must be exactly 8 characters long
const PALLET_ID: ModuleId = ModuleId(*b"Charity!");

pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// The currency type that the charity deals in
    type Currency: Currency<Self::AccountId>;
}

decl_storage! {
    trait Store for Module<T: Trait> as SimpleTreasury {
		// No storage items of our own, but we still need decl_storage to initialize the pot
	}
    add_extra_genesis {
        build(|_config| {
            // Create the charity's pot of funds, and ensure it has the minimum required deposit
            let _ = T::Currency::make_free_balance_be(
                &<Module<T>>::account_id(),
                T::Currency::minimum_balance(),
            );
        });
    }
}

decl_event!(
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        <T as system::Trait>::AccountId,
    {
		/// Donor has made a charitable donation to the charity
		DonationReceived(AccountId, Balance, Balance),
        /// An imbalance from elsewhere in the runtime has been absorbed by the Charity
		ImbalanceAbsorbed(Balance, Balance),
		/// Charity has allocated funds to a cause
		FundsAllocated(AccountId, Balance, Balance),
        /// For testing purposes, to impl From<()> for TestEvent to assign `()` to balances::Event
		/// TODO Do we even need this?
        NullEvent(u32), // u32 could be aliases as an error code for mocking setup
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Donate some funds to the charity
        fn donate(
            origin,
            amount: BalanceOf<T>
        ) -> DispatchResult {
            let donor = ensure_signed(origin)?;

            let _ = T::Currency::transfer(&donor, &Self::account_id(), amount, AllowDeath);

            Self::deposit_event(RawEvent::DonationReceived(donor, amount, Self::pot()));
            Ok(())
        }

        /// Allocate the Charity's funds
		///
        /// Take funds from the Charity's pot and send them somewhere. This cal lrequires root origin,
		/// which means it must come from a governance mechanism such as Substrate's Democracy pallet.
        fn allocate(
            origin,
            dest: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;

			// Make the transfer requested
			let _ = T::Currency::transfer(
				&Self::account_id(),
				&dest,
				amount,
				AllowDeath,
			);

			//TODO what about errors here??

            Self::deposit_event(RawEvent::FundsAllocated(dest, amount, Self::pot()));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    /// The account ID that holds the Charity's funds
    pub fn account_id() -> T::AccountId {
        PALLET_ID.into_account()
    }

    /// The Charity's balance
    fn pot() -> BalanceOf<T> {
        T::Currency::free_balance(&Self::account_id())
    }
}

// This implementation allows the charity to be the recipient of funds that are burned elsewhere in
// the runtime. For eample, it could be transaction fees, consensus-related slashing, or burns that
// align incentives in other pallets.
impl<T: Trait> OnUnbalanced<NegativeImbalanceOf<T>> for Module<T> {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::Currency::resolve_creating(&Self::account_id(), amount);

		Self::deposit_event(RawEvent::ImbalanceAbsorbed(numeric_amount, Self::pot()));
	}
}

#[cfg(test)]
mod tests {
    use crate::*;
    use balances;
    use sp_core::H256;
    use sp_io;
    use sp_runtime::{
        testing::Header,
        traits::{BlakeTwo256, IdentityLookup},
        Perbill,
    };
    use support::{assert_ok, assert_err, impl_outer_event, impl_outer_origin, parameter_types};

    impl_outer_origin! {
        pub enum Origin for TestRuntime {}
    }

    // Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct TestRuntime;
    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: u32 = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::one();

        pub const ExistentialDeposit: u64 = 0;
        pub const TransferFee: u64 = 0;
        pub const CreationFee: u64 = 0;
    }
    impl system::Trait for TestRuntime {
        type Origin = Origin;
        type Index = u64;
        type Call = ();
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = TestEvent;
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type ModuleToIndex = ();
    }

    impl balances::Trait for TestRuntime {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type Event = ();
        type TransferPayment = ();
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type TransferFee = TransferFee;
        type CreationFee = CreationFee;
    }

    mod charity {
        pub use crate::Event;
    }

    impl_outer_event! {
        pub enum TestEvent for TestRuntime {
            charity<T>,
        }
    }

    impl std::convert::From<()> for TestEvent {
        fn from(_unit: ()) -> Self {
            TestEvent::charity(RawEvent::NullEvent(6))
        }
    }

    impl Trait for TestRuntime {
        type Event = TestEvent;
        type Currency = balances::Module<Self>;
    }

    pub type System = system::Module<TestRuntime>;
    pub type Balances = balances::Module<TestRuntime>;
    pub type Charity = Module<TestRuntime>;

    // An alternative to `ExtBuilder` which includes custom configuration
    pub fn new_test_ext() -> sp_io::TestExternalities {
        let mut t = system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();
        balances::GenesisConfig::<TestRuntime> {
			// Provide some initial balances
            balances: vec![
                (1, 13),
                (2, 11),
                (3, 1),
                (4, 3),
                (5, 19),
            ],
            vesting: vec![],
        }
        .assimilate_storage(&mut t)
        .unwrap();
        t.into()
    }

    /// Verifying correct behavior of boilerplate
    #[test]
    fn new_test_ext_behaves() {
        new_test_ext().execute_with(|| {
            assert_eq!(Balances::free_balance(&1), 13);
        })
    }

    /// Transfer reserves tax == 2
    #[test]
    fn transfer_reserves_tax() {
        new_test_ext().execute_with(|| {
            assert_err!(
                Treasury::request_transfer(Origin::signed(3), 1, 1),
                "Must be able to pay tax to make transfer"
            );
            assert_ok!(Treasury::request_transfer(Origin::signed(1), 2, 8));
            assert_eq!(Balances::reserved_balance(&1), 2);
            let mock_spend_request = SpendRequest {
                from: 1,
                to: 2,
                amount: 8, // Balances::from()
            };
            // check that the expected spend request is in runtime storage
            assert!(Treasury::transfer_requests()
                .iter()
                .any(|a| *a == mock_spend_request));

            // check that user debt is correctly tracked
            assert_eq!(Treasury::user_debt(&1).unwrap(), 8,);

            // check that the correct event is emitted
            let expected_event = TestEvent::treasury(RawEvent::TransferRequested(
                1,
                2,
                8,
            ));
            assert!(System::events().iter().any(|a| a.event == expected_event));
        })
    }

    #[test]
    fn propose_treasury_spend_works() {
        new_test_ext().execute_with(|| {
            assert_err!(
                Treasury::propose_treasury_spend(Origin::signed(8), 1, 10u64.into()),
                "must be on council to make proposal"
            );
            System::set_block_number(5);
            assert_ok!(Treasury::propose_treasury_spend(Origin::signed(1), 8, 10u64.into()));

            let expected_proposal = Proposal {
                to: 8,
                amount: 10u64.into(),
                when: 5u64.into(),
                support: 1u32,
            };
            assert_eq!(
                Treasury::proposals(1).unwrap(),
                expected_proposal
            );

            let expected_event = TestEvent::treasury(RawEvent::TreasuryProposal(
                8,
                10u64.into(),
            ));
            assert!(System::events().iter().any(|a| a.event == expected_event));
        })
    }

    // TODO: test
    // - user_spend and expected behavior in different environments with `on_finalize`
    // - treasury_spend and expected behavior in different environments with `on_finalize`
    // - both in different order (need to test all possible overlapping configurations, maybe in a model checker like TLA+)
}
