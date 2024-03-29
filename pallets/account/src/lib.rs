#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::inherent::Vec;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
//use pallet_fund_raising::{Role, Status};
use scale_info::TypeInfo;

use serde::{Deserialize, Serialize};

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	pub use super::*;
	
	// https://docs.substrate.io/reference/how-to-guides/basics/configure-genesis-state/
	#[derive(Encode, Decode, Ord, PartialOrd, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Role {
		SysMan,
		Voter,
		User,
	}

	impl Default for Role {
		fn default() -> Self {
			Self::User
		}
	}

	#[derive(Encode, Decode, Ord, PartialOrd, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Status {
		Active,
		Revoked,
		Pending,
	}

	impl Default for Status {
		fn default() -> Self {
			Self::Pending
		}
	}

	// score request
	#[derive(Encode, Decode, Ord, PartialOrd, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Flag {
		On,
		Off,
	}

	impl Default for Flag {
		fn default() -> Self {
			Self::Off
		}
	}

	// erase  validator
	/*
	#[derive(Encode, Decode, Ord, PartialOrd, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Valid {
		Validated,
		Unvalidated,
	}

	impl Default for Valid {
		fn default() -> Self {
			Self::Unvalidated
		}
	}
	*/

	#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(bounds(), skip_type_params(T))]
	pub struct Account<T: Config> {
		pub id: T::AccountId,
		pub role: Role,
		pub status: Status,
		pub flag: Flag,
		pub metadata: Vec<u8>,
		pub score: i32,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn account_storage)]
	pub type AccountStorage<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Account<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn account_role)]
	pub type AccountRole<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Role, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub sysman_accountmap: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { sysman_accountmap: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for a in &self.sysman_accountmap {
				let _a = a.clone();
				let account = Account::<T> {
					id: _a,
					role: Role::SysMan,
					status: Status::Active,
					flag: Flag::Off,
					metadata: Vec::new(),
					score: 500,
				};
				<AccountStorage<T>>::insert(a, account);
				<AccountRole<T>>::insert(a, Role::SysMan);
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AccountRegisted(T::AccountId),
		SysmanRegisted(T::AccountId),
		VoterRegisted(T::AccountId),
		// VoterRevoked(T::AccountId),
		UserRevoked(T::AccountId),
		AccountUpdated(T::AccountId),
		VoterRequest(T::AccountId),
		VoterRegistered(T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Account is already Registered
		AlreadyRegistered,
		/// Account is not Registered
		AccountNotRegistered,

		InvalidAccount,

		NotExactRole,

		NotExactStatus,

		NotExactValid,

		NotEnoughScore,

		NotRequestVoter,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]

		//　Todo: claim voter 追加！！！
		pub fn register_account(origin: OriginFor<T>, metadata: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			match <AccountStorage<T>>::try_get(&who) {
				Err(_) => {
					<AccountStorage<T>>::insert(
						&who,
						Account {
							id: who.clone(),
							role: Role::User,
							status: Status::Active,
							flag: Flag::Off,
							metadata,
							score: 100,
						},
					);
					<AccountRole<T>>::insert(&who, Role::User);
				},
				Ok(_) => Err(Error::<T>::AlreadyRegistered)?,
			}
			// Return a successful DispatchResultWithPostInfo
			Self::deposit_event(Event::AccountRegisted(who));
			Ok(())
		}

		// voter request by score
		#[pallet::weight(10_000)]
		pub fn voter_request(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::ensure_role(&who, Role::User)?;
			Self::ensure_status(&who, Status::Active)?;

			let account = <AccountStorage<T>>::get(&who).ok_or(Error::<T>::InvalidAccount)?;

			ensure!(account.score >= 500, Error::<T>::NotEnoughScore);

			<AccountStorage<T>>::try_mutate(&who, |acc| {
				if let Some(account) = acc {
					account.flag = Flag::On;
				} else {
					return Err(Error::<T>::AccountNotRegistered)
				}
				Ok(())
			})?;

			Self::deposit_event(Event::VoterRequest(who));
			Ok(())
		}

		// check voter_request by sysman
		#[pallet::weight(10_000)]
		pub fn votercheck_sysmen(origin: OriginFor<T>, user: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::ensure_role(&who, Role::SysMan)?;
			Self::ensure_status(&who, Status::Active)?;

			let account = <AccountStorage<T>>::get(&user).ok_or(Error::<T>::InvalidAccount)?;

			ensure!(account.flag == Flag::On, Error::<T>::NotRequestVoter);

			<AccountStorage<T>>::try_mutate(&user, |acc| {
				if let Some(account) = acc {
					account.role = Role::Voter;
				} else {
					return Err(Error::<T>::AccountNotRegistered)
				}
				Ok(())
			})?;

			Self::deposit_event(Event::VoterRegistered(user));
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn approve_sysman(origin: OriginFor<T>, sys: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::ensure_role(&who, Role::SysMan)?;
			Self::ensure_status(&who, Status::Active)?;

			<AccountStorage<T>>::try_mutate(&sys, |acc| {
				if let Some(account) = acc {
					account.role = Role::SysMan;
				} else {
					return Err(Error::<T>::AccountNotRegistered)
				}
				Ok(())
			})?;
			<AccountRole<T>>::try_mutate(&sys, |acc| {
				if let Some(account) = acc {
					*account = Role::SysMan;
				} else {
					return Err(Error::<T>::AccountNotRegistered)
				}
				Ok(())
			})?;

			Self::deposit_event(Event::SysmanRegisted(who));
			Ok(())
		}

		// erase approve_voter func
		// due to creating votercheck_sysman func
		// #[pallet::weight(10_000)]
		// pub fn approve_voter(origin: OriginFor<T>, sys: T::AccountId) -> DispatchResult {
		// 	let who = ensure_signed(origin)?;

		// 	Self::ensure_role(&who, Role::SysMan)?;
		// 	Self::ensure_status(&who, Status::Active)?;

		// 	<AccountStorage<T>>::try_mutate(&sys, |acc| {
		// 		if let Some(account) = acc {
		// 			account.role = Role::Voter;
		// 		} else {
		// 			return Err(Error::<T>::AccountNotRegistered)
		// 		}
		// 		Ok(())
		// 	})?;
		// 	<AccountRole<T>>::try_mutate(&sys, |acc| {
		// 		if let Some(account) = acc {
		// 			*account = Role::Voter;
		// 		} else {
		// 			return Err(Error::<T>::AccountNotRegistered)
		// 		}
		// 		Ok(())
		// 	})?;

		// 	Self::deposit_event(Event::VoterRegisted(who));
		// 	Ok(())
		// }

		#[pallet::weight(10_000)]
		pub fn revoke_user(origin: OriginFor<T>, val: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::ensure_status(&who, Status::Active)?;

			Self::ensure_role(&val, Role::User)?;
			Self::ensure_status(&val, Status::Active)?;

			<AccountStorage<T>>::try_mutate(&val, |acc| {
				if let Some(account) = acc {
					account.status = Status::Revoked;
				} else {
					return Err(Error::<T>::AccountNotRegistered)
				}
				Ok(())
			})?;

			Self::deposit_event(Event::UserRevoked(who));
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn update_account(origin: OriginFor<T>, metadata: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<AccountStorage<T>>::try_mutate(&who, |acc| {
				if let Some(account) = acc {
					account.metadata = metadata;
				} else {
					return Err(Error::<T>::AccountNotRegistered)
				}
				Ok(())
			})?;
			// Return a successful DispatchResultWithPostInfo
			Self::deposit_event(Event::AccountUpdated(who));
			Ok(())
		}
	}
}

/* ----------------------------------------------------------- helper function -------------------------------------------------------- */
pub trait EnsureAccount<T: Config> {
	fn ensure_role(who: &T::AccountId, role: Role) -> DispatchResult;
	fn ensure_status(who: &T::AccountId, status: Status) -> DispatchResult;
	// fn check_sysman(who: &T::AccountId) -> DispatchResult;
}

impl<T: Config> EnsureAccount<T> for Pallet<T> {
	// check account's role
	fn ensure_role(who: &T::AccountId, role: Role) -> DispatchResult {
		if let Some(account) = <AccountStorage<T>>::get(who) {
			if account.role == role {
				Ok(())
			} else {
				return Err(Error::<T>::NotExactRole)?
			}
		} else {
			return Err(Error::<T>::AccountNotRegistered)?
		}
	}

	// check account's status
	fn ensure_status(who: &T::AccountId, status: Status) -> DispatchResult {
		if let Some(account) = <AccountStorage<T>>::get(who) {
			if account.status == status {
				Ok(())
			} else {
				return Err(Error::<T>::NotExactStatus)?
			}
		} else {
			return Err(Error::<T>::AccountNotRegistered)?
		}
	}
	/*
	fn check_sysman(who: &T::AccountId) -> DispatchResult{
		let role = Self::account_role(who).ok_or(Error::<T>::InvalidAccount)?;

		if role == Role::SysMan {
			return Ok(())
		} else {
			return Err(Error::<T>::InvalidAccount)?
		}
	}
	*/

}