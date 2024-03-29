#![cfg_attr(not(feature = "std"), no_std)]

// use frame_support::inherent::Vec;
use frame_support::pallet_prelude::*;
use frame_support::traits::{Currency, ReservableCurrency};
use frame_system::pallet_prelude::*;
use pallet_account::AccountStorage;
use pallet_fund_raising::{EnsureRaising, Funds};
use scale_info::prelude::vec;
//use pallet_fund_raising::{Role, Status};
//use scale_info::TypeInfo;

//use serde::{Deserialize, Serialize};

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	pub use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_account::Config + pallet_fund_raising::Config
	{
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: ReservableCurrency<Self::AccountId>;

		type Fee: Get<BalanceOf<Self>>;

		type CheckRate: EnsureRaising<Self>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	pub type FundIndex = u32;

	type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

	/*
	#[pallet::storage]
	#[pallet::getter(fn score_storage)]
	pub type ScoreStorage<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, i32, OptionQuery>;
	*/

	#[pallet::storage]
	#[pallet::getter(fn amount)]
	// initail value : 0
	pub type Amount<T> = StorageValue<_, i32>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub amount_num: i32,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self { amount_num: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<Amount<T>>::put(&self.amount_num);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ScoreSet(T::AccountId, i32),

		ScoreCheck(T::AccountId, i32),
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadySet,
		NotEnoughAmount,
		InvalidAccount,
		InvalidIndex,
		InvalidScore,
		AccountNotRegistered,
		CannotEvaluate,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/*
		#[pallet::weight(10_000)]
		pub fn set_score(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// ensure!(T::Currency::total_balance(&who) >= T::Fee::get(), Error::<T>::NotEnoughAmount);

			// need to pay Fee to use this "set_score" function
			// ToDo : write transport function
			/*
			match <ScoreStorage<T>>::try_get(&who) {
				Err(_) => {
					<ScoreStorage<T>>::insert(&who, 100);
				},
				Ok(_) => Err(Error::<T>::AlreadySet)?,
			}
			*/

			Self::deposit_event(Event::ScoreSet(who, 100));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}
		*/

		#[pallet::weight(10_000)]
		pub fn evaluation(
			origin: OriginFor<T>,
			index: FundIndex,
			val: AccountIdOf<T>,
			mut rate: i32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let sender = <AccountStorage<T>>::get(&who).ok_or(Error::<T>::InvalidAccount)?;
			let receiver = <AccountStorage<T>>::get(&val).ok_or(Error::<T>::InvalidAccount)?;

			// Todo
			// ------------------------ contribution_check ----------------------//
			// after crowdfund_kill, you cannot evaluate (because you cannot check "Funds Storage")
			T::CheckRate::contribution_check(&who, index)?;

			let fund = <Funds<T>>::get(&index).ok_or(Error::<T>::InvalidIndex)?;

			// check "sender == crater && receiver == contributer" or "sender == contributer && receiver == crater"
			// check sender != receiver
			ensure!(&fund.creater != &who && &fund.creater != &val, Error::<T>::CannotEvaluate);
			ensure!(&who != &val, Error::<T>::CannotEvaluate);

			let sender_score = sender.score;
			let receiver_score = receiver.score;

			//let receiver_score = Self::score_storage(&account).ok_or(Error::<T>::InvalidAccount)?;
			//let sender_score = Self::score_storage(&who).ok_or(Error::<T>::InvalidAccount)?;

			rate += 1;
			ensure!(rate >= 1 && rate <= 6, Error::<T>::InvalidScore);

			// 1: -3, 2: -2, 3: -1, 4: 1, 5: 2, 6: 3
			let psc = vec![-3, -2, -1, 1, 2, 3];

			let mut calc =
				(receiver_score + psc[rate as usize]) + (psc[rate as usize] * sender_score) / 100;

			if calc >= 1000 {
				calc = 1000;
			} else if calc < 0 {
				calc = 0;
			}

			<AccountStorage<T>>::try_mutate(&val, |acc| {
				if let Some(account) = acc {
					account.score = calc;
				} else {
					return Err(Error::<T>::AccountNotRegistered);
				}
				Ok(())
			})?;

			Self::deposit_event(Event::ScoreCheck(val, calc));

			Ok(())
		}

		// check other account score
		#[pallet::weight(10_000)]
		pub fn check_score(origin: OriginFor<T>, val: AccountIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// check origin account is registered
			ensure!(<AccountStorage<T>>::contains_key(&who) == true, Error::<T>::InvalidAccount);
			// check "account id" is registered
			let receiver = <AccountStorage<T>>::get(&val).ok_or(Error::<T>::InvalidAccount)?;
			let receiver_score = receiver.score;

			Self::deposit_event(Event::ScoreCheck(val, receiver_score));

			Ok(())
		}
	}
}
