#![cfg_attr(not(feature = "std"), no_std)]

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
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use frame_support::{
		inherent::Vec,
		storage::child,
		traits::{Currency, ExistenceRequirement, ReservableCurrency, WithdrawReasons},
		PalletId,
	};

	// use serde::{Deserialize, Serialize};

	use sp_runtime::traits::{AccountIdConversion, Hash, One, Saturating, Zero};

	use scale_info::TypeInfo;

	use pallet_account::{AccountStorage, EnsureAccount, Role};

	const PALLET_ID: PalletId = PalletId(*b"ex/cfund");

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_account::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: ReservableCurrency<Self::AccountId>;

		type SubmissionDeposit: Get<BalanceOf<Self>>;

		type MinContribution: Get<BalanceOf<Self>>;

		type FeePercent: Get<BalanceOf<Self>>;

		type Percent: Get<BalanceOf<Self>>;

		type MinVotenum: Get<BalanceOf<Self>>;

		type RetirementPeriod: Get<Self::BlockNumber>;

		type VotingPeriod: Get<Self::BlockNumber>;

		type CheckEnsure: EnsureAccount<Self>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Simple index for identifying a fund.
	pub type FundIndex = u32;

	type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
	type FundInfoOf<T> =
		FundInfo<AccountIdOf<T>, BalanceOf<T>, <T as frame_system::Config>::BlockNumber>;

	#[derive(Encode, Decode, Default, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct FundInfo<AccountId, Balance, BlockNumber> {
		fundnum: u32,
		/// The account that creat
		pub creater: AccountId,
		/// The account that will recieve the funds if the campaign is successful
		beneficiary: AccountId,
		/// The amount of deposit placed
		deposit: Balance,
		/// The total amount raised
		raised: Balance,
		/// Block number which funding created
		start: BlockNumber,
		/// Block number after which funding must have succeeded
		/// Length of block time from creation to end of funding
		end: BlockNumber,
		/// Upper bound on `raised`
		goal: Balance,
		/// The total amount voted
		totalvote: Balance,
		/// whether fund gets enough votes
		voteflag: bool,
	}

	#[pallet::storage]
	#[pallet::getter(fn funds)]
	pub type Funds<T: Config> =
		StorageMap<_, Blake2_128Concat, FundIndex, FundInfoOf<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn fund_count)]
	pub type FundCount<T: Config> = StorageValue<_, FundIndex, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn memberlist)]
	pub type Memberlist<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		FundIndex,
		Blake2_128Concat,
		BalanceOf<T>,
		AccountIdOf<T>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	/// #[pallet::metadata(BalanceOf<T> = "Balance", AccountId<T> = "AccountId", BlockNumber<T> = "BlockNumber")]
	pub enum Event<T: Config> {
		Created(
			FundIndex,
			<T as frame_system::Config>::BlockNumber,
			<T as frame_system::Config>::BlockNumber,
		),
		Voted(
			<T as frame_system::Config>::AccountId,
			FundIndex,
			BalanceOf<T>,
			<T as frame_system::Config>::BlockNumber,
		),
		Contributed(
			<T as frame_system::Config>::AccountId,
			FundIndex,
			BalanceOf<T>,
			<T as frame_system::Config>::BlockNumber,
		),
		Withdrew(
			<T as frame_system::Config>::AccountId,
			FundIndex,
			BalanceOf<T>,
			<T as frame_system::Config>::BlockNumber,
		),
		Retiring(FundIndex, <T as frame_system::Config>::BlockNumber),
		Dissolved(
			FundIndex,
			<T as frame_system::Config>::BlockNumber,
			<T as frame_system::Config>::AccountId,
		),
		Dispensed(
			FundIndex,
			<T as frame_system::Config>::BlockNumber,
			<T as frame_system::Config>::AccountId,
		),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Crowdfund must end after it starts
		EndTooEarly,
		/// Must contribute at least the minimum amount of funds
		ContributionTooSmall,
		/// The fund index specified does not exist
		InvalidIndex,
		/// The account does not exist
		InvalidAccount,
		/// The crowdfund's contribution period has ended; no more contributions will be accepted
		ContributionPeriodOver,
		/// You may not withdraw or dispense funds while the fund is still active
		FundStillActive,
		/// You may not contribute funds while the fund is still active
		VoteStillActive,
		/// You cannot withdraw funds because you have not contributed any
		NoContribution,
		/// You cannot dissolve a fund that has not yet completed its retirement period
		FundNotRetired,
		/// Cannot dispense funds from an unsuccessful fund
		UnsuccessfulFund,
		/// You already voted to a fund
		Alreadyvoted,
		/// You cant contribute beacuse of it doesnt meet number of votes
		CannotContribution,
		/// Too late to vote
		CannnotVote,

		NotEnoughAmount,

		NotEnoughScore,

		NotVoterAccount,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create(
			origin: OriginFor<T>,
			beneficiary: AccountIdOf<T>,
			goal: BalanceOf<T>,
			mut end: T::BlockNumber,
		) -> DispatchResultWithPostInfo {
			let creater = ensure_signed(origin)?;
			let _account = <AccountStorage<T>>::get(&creater).ok_or(Error::<T>::InvalidAccount)?;

			let start = <frame_system::Pallet<T>>::block_number();
			ensure!(end > Zero::zero(), Error::<T>::EndTooEarly);
			let deposit = T::SubmissionDeposit::get();
			let imb = T::Currency::withdraw(
				&creater,
				deposit,
				WithdrawReasons::TRANSFER,
				ExistenceRequirement::AllowDeath,
			)?;

			end += T::VotingPeriod::get() + start;

			let index = <FundCount<T>>::get();
			// not protected against overflow, see safemath section
			<FundCount<T>>::put(index + 1);
			// No fees are paid here if we need to create this account; that's why we don't just
			// use the stock `transfer`.
			T::Currency::resolve_creating(&Self::fund_account_id(index), imb);

			<Funds<T>>::insert(
				index,
				FundInfo {
					fundnum: index,
					creater,
					beneficiary,
					deposit,
					raised: Zero::zero(),
					start,
					end,
					goal,
					totalvote: Zero::zero(),
					voteflag: false,
				},
			);

			Self::deposit_event(Event::Created(index, start, end));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		pub fn contribute(
			origin: OriginFor<T>,
			index: FundIndex,
			value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			// let account = <AccountStorage<T>>::get(&who).ok_or(Error::<T>::InvalidAccount)?;

			ensure!(value >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);
			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex)?;
			ensure!(fund.totalvote >= T::MinVotenum::get(), Error::<T>::CannotContribution);
			ensure!(fund.voteflag == true, Error::<T>::CannotContribution);

			// Make sure crowdfund has not ended
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(now > fund.start + T::VotingPeriod::get(), Error::<T>::VoteStillActive);
			ensure!(fund.end > now, Error::<T>::ContributionPeriodOver);

			// Add contribution to the fund
			T::Currency::transfer(
				&who,
				&Self::fund_account_id(index),
				value,
				ExistenceRequirement::AllowDeath,
			)?;
			fund.raised += value;
			Funds::<T>::insert(index, &fund);

			let balance = Self::contribution_get(index, &who);
			let balance = balance.saturating_add(value);
			Self::contribution_put(index, &who, &balance);

			Self::deposit_event(Event::Contributed(who, index, balance, now));

			Ok(().into())
		}

		// ToDo : vote の期限の設定
		// ensure!(fund.totalvote >= T::MinVotenum::get(), Error::<T>::CannotContribution);
		#[pallet::weight(10_000)]
		pub fn vote(origin: OriginFor<T>, index: FundIndex) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let account = <AccountStorage<T>>::get(&who).ok_or(Error::<T>::InvalidAccount)?;

			// check limit
			// "200" >= score holder can vote
			ensure!(account.score >= 200, Error::<T>::NotEnoughScore);

			// ToDo
			// ToDo!!!
			// Role::Voterの部分をコメントアウトしているので、処理
			// T::CheckEnsure::ensure_role(&who, Role::Voter)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex)?;
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(fund.start + T::VotingPeriod::get() > now, Error::<T>::CannnotVote);

			// let num = Self::vote_get(index, &who);
			// vote check
			//ensure!(num == 0, Error::<T>::Alreadyvoted);

			Memberlist::<T>::insert(index, fund.totalvote, &who);

			fund.totalvote += One::one();

			Self::vote_put(index, &who);

			if fund.totalvote >= T::MinVotenum::get() && fund.voteflag == false {
				fund.voteflag = true;
			}

			Funds::<T>::insert(index, &fund);

			Self::deposit_event(Event::Voted(who, index, fund.totalvote, now));

			Ok(().into())
		}

		#[pallet::weight(10_000)]
		pub fn withdraw(
			origin: OriginFor<T>,
			#[pallet::compact] index: FundIndex,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex)?;

			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(fund.end < now, Error::<T>::FundStillActive);

			let balance = Self::contribution_get(index, &who);
			ensure!(balance > Zero::zero(), Error::<T>::NoContribution);

			// Return funds to caller without charging a transfer fee
			let _ = T::Currency::resolve_into_existing(
				&who,
				T::Currency::withdraw(
					&Self::fund_account_id(index),
					balance,
					WithdrawReasons::TRANSFER,
					ExistenceRequirement::AllowDeath,
				)?,
			);

			// Update storage
			Self::contribution_kill(index, &who);
			fund.raised = fund.raised.saturating_sub(balance);
			<Funds<T>>::insert(index, &fund);

			Self::deposit_event(Event::Withdrew(who, index, balance, now));

			Ok(().into())
		}

		#[pallet::weight(10_000)]
		pub fn dissolve(origin: OriginFor<T>, index: FundIndex) -> DispatchResultWithPostInfo {
			let reporter = ensure_signed(origin)?;
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex)?;

			// Check that enough time has passed to remove from storage
			let now = <frame_system::Pallet<T>>::block_number();
			//ensure!(now > fund.end + T::RetirementPeriod::get(), Error::<T>::FundNotRetired);

			let account = Self::fund_account_id(index);

			// Dissolver collects the deposit and any remaining funds
			let _ = T::Currency::resolve_creating(
				&reporter,
				T::Currency::withdraw(
					&account,
					fund.deposit + fund.raised,
					WithdrawReasons::TRANSFER,
					ExistenceRequirement::AllowDeath,
				)?,
			);

			// Remove the fund info from storage
			<Funds<T>>::remove(index);
			// Remove all the contributor info from storage in a single write.
			// This is possible thanks to the use of a child tree.
			Self::crowdfund_kill(index);

			Self::deposit_event(Event::Dissolved(index, now, reporter));

			Ok(().into())
		}

		#[pallet::weight(10_000)]
		// block数が超える　and 金額が目標を超えたときのみ使える
		pub fn dispense(origin: OriginFor<T>, index: FundIndex) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex)?;

			// Check that enough time has passed to remove from storage
			let now = <frame_system::Pallet<T>>::block_number();

			ensure!(now > fund.end, Error::<T>::FundStillActive);

			// Check that the fund was actually successful
			ensure!(fund.raised >= fund.goal, Error::<T>::UnsuccessfulFund);

			let account = Self::fund_account_id(index);

			// voter に渡される金額
			let mut voteramount = (fund.raised / T::FeePercent::get()) / fund.totalvote;
			// beneficiary に渡される金額
			let beneficiaryamount = fund.raised - voteramount * fund.totalvote;

			voteramount -= voteramount / T::Percent::get();

			// Beneficiary collects the contributed funds
			let _ = T::Currency::resolve_creating(
				&fund.beneficiary,
				T::Currency::withdraw(
					&account,
					beneficiaryamount,
					WithdrawReasons::TRANSFER,
					ExistenceRequirement::AllowDeath,
				)?,
			);

			let mut balancenum: BalanceOf<T> = Zero::zero();

			// give monry who votes to this idea
			while balancenum != fund.totalvote {
				let _account =
					Self::memberlist(index, &balancenum).ok_or(Error::<T>::InvalidIndex)?;
				let _ = T::Currency::resolve_creating(
					&_account,
					T::Currency::withdraw(
						&account,
						voteramount,
						WithdrawReasons::TRANSFER,
						ExistenceRequirement::AllowDeath,
					)?,
				);
				balancenum += One::one();
			}

			// Caller collects the deposit
			let _ = T::Currency::resolve_creating(
				&caller,
				T::Currency::withdraw(
					&account,
					fund.deposit,
					WithdrawReasons::TRANSFER,
					ExistenceRequirement::AllowDeath,
				)?,
			);

			// Remove the fund info from storage
			<Funds<T>>::remove(index);
			// Remove all the contributor info from storage in a single write.
			// This is possible thanks to the use of a child tree.
			Self::crowdfund_kill(index);

			Self::deposit_event(Event::Dispensed(index, now, caller));
			Ok(().into())
		}
	}

	/* ----------------------------------------------------------- helper function -------------------------------------------------------- */
	pub trait EnsureRaising<T: Config> {
		fn contribution_check(who: &T::AccountId, index: FundIndex) -> DispatchResult;
	}

	impl<T: Config> EnsureRaising<T> for Pallet<T> {
		fn contribution_check(who: &T::AccountId, index: FundIndex) -> DispatchResult {
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex)?;

			let now = <frame_system::Pallet<T>>::block_number();
			if fund.end >= now {
				return Err(Error::<T>::FundStillActive)?
			} 

			let balance = Self::contribution_get(index, &who);
			if balance <= Zero::zero() {
				return Err(Error::<T>::NoContribution)?
			}
			
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID of the fund pot.
		///
		/// This actually does computation. If you need to keep using it, then make sure you cache the
		/// value and only call this once.
		pub fn fund_account_id(index: FundIndex) -> T::AccountId {
			PALLET_ID.into_sub_account(index)
		}

		/// Find the ID associated with the fund
		///
		/// Each fund stores information about its contributors and their contributions in a child trie
		/// This helper function calculates the id of the associated child trie.
		pub fn id_from_index(index: FundIndex) -> child::ChildInfo {
			let mut buf = Vec::new();
			buf.extend_from_slice(b"crowdfnd");
			buf.extend_from_slice(&index.to_le_bytes()[..]);

			child::ChildInfo::new_default(T::Hashing::hash(&buf[..]).as_ref())
		}

		/* ---------------------------------------- vote part --------------------------------------------------- */
		/// Record vote in the associated child trie.
		pub fn vote_put(index: FundIndex, who: &T::AccountId) {
			let id = Self::id_from_index(index);
			who.using_encoded(|b| child::put(&id, b, &1));
		}

		/// Lookup vote in the associated child trie.
		pub fn vote_get(index: FundIndex, who: &T::AccountId) -> BalanceOf<T> {
			let id = Self::id_from_index(index);
			who.using_encoded(|b| child::get_or_default::<BalanceOf<T>>(&id, b))
		}

		/// Remove a contribution from an associated child trie.
		pub fn vote_kill(index: FundIndex, who: &T::AccountId) {
			let id = Self::id_from_index(index);
			who.using_encoded(|b| child::kill(&id, b));
		}

		/* ---------------------------------------- contribution part ----------------------------------------------- */
		/// Record a contribution in the associated child trie.
		pub fn contribution_put(index: FundIndex, who: &T::AccountId, balance: &BalanceOf<T>) {
			let id = Self::id_from_index(index);
			who.using_encoded(|b| child::put(&id, b, &balance));
		}

		/// Lookup a contribution in the associated child trie.
		pub fn contribution_get(index: FundIndex, who: &T::AccountId) -> BalanceOf<T> {
			let id = Self::id_from_index(index);
			who.using_encoded(|b| child::get_or_default::<BalanceOf<T>>(&id, b))
		}

		/// Remove a contribution from an associated child trie.
		pub fn contribution_kill(index: FundIndex, who: &T::AccountId) {
			let id = Self::id_from_index(index);
			who.using_encoded(|b| child::kill(&id, b));
		}

		/// Remove the entire record of contributions in the associated child trie in a single
		/// storage write.
		pub fn crowdfund_kill(index: FundIndex) {
			let id = Self::id_from_index(index);
			// The None here means we aren't setting a limit to how many keys to delete.
			// Limiting can be useful, but is beyond the scope of this recipe. For more info, see
			// https://crates.parity.io/frame_support/storage/child/fn.kill_storage.html
			child::kill_storage(&id, None);
		}
	}
}
