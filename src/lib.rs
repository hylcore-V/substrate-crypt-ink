// Copyright 2020-2021 OxyDev.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

#[ink_lang::contract]
pub mod subscrypt {
    use core::convert::TryInto;
    use ink_env::hash::Sha2x256;
    use ink_env::Error;
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    use ink_storage::collections::HashMap;
    use ink_storage::traits::{PackedLayout, SpreadLayout};
    use ink_prelude::vec;
    /// This struct represents a subscription record
    /// # fields:
    /// * provider
    /// * plan
    /// * plan_index
    /// * subscription_time : this stores start time of each subscription (used in linkedList)
    /// * meta_data_encrypted
    /// * refunded
    #[derive(
        scale::Encode, scale::Decode, SpreadLayout, PackedLayout, Debug,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct SubscriptionRecord {
        pub provider: AccountId,
        pub plan: PlanConsts,
        pub plan_index: u128,
        subscription_time: u64,
        charastristics_values_encrypted: Vec<String>,
        //encrypted Data with public key of provider
        pub refunded: bool,
    }

    /// This struct stores user plan records
    /// # fields:
    /// * subscription_records
    /// * pass_hash : hash of (token + pass_phrase) for authenticating user without wallet
    #[derive(
        scale::Encode, scale::Decode, SpreadLayout, PackedLayout, Debug,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PlanRecord {
        pub subscription_records: Vec<SubscriptionRecord>,
        pass_hash: [u8; 32],
    }

    /// This struct stores configs of plan which is set by provider
    /// # Note
    /// `max_refund_permille_policy` is out of 1000
    #[derive(
        scale::Encode,
        scale::Decode,
        PackedLayout,
        SpreadLayout,
        Debug,
        Clone,
        Copy,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PlanConsts {
        pub duration: u64,
        pub(crate) active_session_limit: u128,
        pub(crate) price: u128,
        pub(crate) max_refund_permille_policy: u128,
        pub disabled: bool,
    }

    /// This struct represents a provider
    /// # fields:
    /// * plans
    /// * money_address : provider earned money will be sent to this address
    /// * payment_manager : struct for handling refund requests
    #[derive(
        scale::Encode, scale::Decode, PackedLayout, SpreadLayout, Debug,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Provider {
        pub(crate) plans: Vec<PlanConsts>,
        pub(crate) plans_charastristics: Vec<Vec<String>>,
        pub(crate) money_address: AccountId,
        payment_manager: LinkedList,
        pub subs_crypt_pass_hash: [u8; 32],
    }

    /// This struct represents a user
    /// # fields:
    /// * list_of_providers : list of providers that the user subscribed to
    /// * subs_crypt_pass_hash : pass hash for retrieve data in subscrypt user dashboard
    #[derive(
        scale::Encode, scale::Decode, SpreadLayout, PackedLayout, Debug, 
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct User {
        pub list_of_providers: Vec<AccountId>,
        pub subs_crypt_pass_hash: [u8; 32],
    }

    /// Struct for handling payments of refund
    /// # Description
    ///
    /// This LinkedList is used for keeping tracking of each subscription that will end in some
    /// specific date in future. We order these subscriptions by their date of expiration, so we
    /// will be able to easily calculate and handle refund - withdraw methods with a minimum
    /// transaction fee. Each entity of the linked-list is `PaymentAdmission` struct.
    #[derive(
        scale::Encode, scale::Decode, PackedLayout, SpreadLayout, Debug,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct LinkedList {
        pub head: u64,
        pub back: u64,
        pub length: u128,
    }

    /// Struct that represents amount of money that can be withdraw after its due date passed.
    #[derive(
        scale::Encode, scale::Decode, PackedLayout, SpreadLayout, Debug,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct DailyLockedAmount {
        amount: u128,
        next_day: u64,
    }

    pub struct ProcessReturningData {
        withdrawing_amount : u128,
        current_linked_list_head : u64,
        reduced_length : u128,
    }

    /// Main struct of contract
    /// # fields:
    /// * `start_time` : start time of the contract which is used in `LinkedList`
    /// * `provider_register_fee`
    /// * `providers` : the hashmap that stores providers data
    /// * `users` : the hashmap that stores users data
    /// * `daily_locked_amounts` : the hashmap that stores `DailyLockedAmount` data of each day in order
    /// * `records` : the hashmap that stores user's subscription records data
    /// * `plan_index_to_record_index` : the hashmap that stores user's last `SubscriptionRecord` index
    /// in `PlanRecord.subscription_records` for each (user, provider, plan_index)
    #[ink(storage)]
    pub struct Subscrypt {
        start_time: u64,
        pub provider_register_fee: u128,
        // (provider AccountId) -> provider data
        pub(crate) providers: HashMap<AccountId, Provider>,
        // (user AccountId) -> user data
        pub users: HashMap<AccountId, User>,
        // (provider AccountId , day_id) -> payment admission
        daily_locked_amounts: HashMap<(AccountId, u64), DailyLockedAmount>,
        // (user AccountId, provider AccountId) -> PlanRecord struct
        pub records: HashMap<(AccountId, AccountId), PlanRecord>,
        // (user AccountId, provider AccountId, plan_index) -> index
        plan_index_to_record_index: HashMap<(AccountId, AccountId, u128), u128>,
        // username -> user AccountId
        username_to_address: HashMap<String, AccountId>,
        // username -> user AccountId
        address_to_username: HashMap<AccountId, String>,
    }

    impl Default for Subscrypt {
        fn default() -> Self {
            Self::new()
        }
    }

    #[ink(event)]
    pub struct ProviderRegisterEvent {
        #[ink(topic)]
        address: AccountId,
    }

    #[ink(event)]
    pub struct AddPlanEvent {
        #[ink(topic)]
        owner: AccountId,
        duration: u64,
        price: u128,
    }

    #[ink(event)]
    pub struct SubscribeEvent {
        #[ink(topic)]
        provider: AccountId,
        #[ink(topic)]
        plan_index: u128,
        subscription_time: u64,
        duration: u64,
    }

    impl Subscrypt {
        #[ink(constructor)]
        pub fn new() -> Self {
            Subscrypt::default()
        }

        #[ink(constructor)]
        pub fn default() -> Self {
            Self {
                start_time: Self::env().block_timestamp(),
                provider_register_fee: 100,
                providers: HashMap::new(),
                users: ink_storage::collections::HashMap::new(),
                daily_locked_amounts: ink_storage::collections::HashMap::new(),
                records: ink_storage::collections::HashMap::new(),
                plan_index_to_record_index: ink_storage::collections::HashMap::new(),
                username_to_address: ink_storage::collections::HashMap::new(),
                address_to_username: ink_storage::collections::HashMap::new(),                
            }
        }

        /// Registering a new `Provider` by paying the required fee amount (`provider_register_fee`)
        ///
        /// # Panics
        ///
        /// If paid amount is less than `provider_register_fee`
        /// if same `AccountId` registered as provider previously.
        ///
        /// # Examples
        /// Examples of different situations in `provider_register_works` , `provider_register_works2` and
        /// `provider_register_works3` in `tests/test.rs`
        #[ink(message, payable)]
        pub fn provider_register(
            &mut self,
            durations: Vec<u64>,
            active_session_limits: Vec<u128>,
            prices: Vec<u128>,
            max_refund_permille_policies: Vec<u128>,
            address: AccountId,
            username: String,
            subs_crypt_pass_hash: [u8; 32],
            plans_charastristics: Vec<Vec<String>>,
        ) {

            assert_eq!(durations.len(), active_session_limits.len(),"Wrong Number of Args");
            assert_eq!(prices.len(), active_session_limits.len(),"Wrong Number of Args");
            assert_eq!(
                max_refund_permille_policies.len(),
                active_session_limits.len(),
                "Wrong Number of Args"
            );
            assert_eq!(
                max_refund_permille_policies.len(),
                plans_charastristics.len(),
                "Wrong Number of Args"
            );

            let caller = self.env().caller();

            match self.address_to_username.get(&caller) {
                Some(_) => {}
                None => {
                    match self.username_to_address.get(&username) {
                        Some(address) => assert_eq!(*address, caller, "this username is invalid!"),
                        None => {}
                    }
                    self.address_to_username.insert(caller, username.clone());
                    self.username_to_address.insert(username, caller);
                }
            }


            assert!(
                self.env().transferred_balance() >= self.provider_register_fee,
                "You have to pay a minimum amount to register in the contract!"
            );
            assert!(
                !self.providers.contains_key(&caller),
                "You can not register again in the contract!"
            );

            let provider = Provider {
                plans: Vec::new(),
                plans_charastristics: Vec::new(),
                money_address: address,
                payment_manager: LinkedList::new(),
                subs_crypt_pass_hash,
            };

            self.providers.insert(caller, provider);
            self.add_plan(
                durations,
                active_session_limits,
                prices,
                max_refund_permille_policies,
                plans_charastristics,
            );
            self.env().emit_event(ProviderRegisterEvent {
                address: address,
            });
        }

        /// Add plans to `provider` storage
        ///
        /// # Panics
        ///
        /// If the size of vectors passed to the methods are different
        /// If the caller is not a valid provider.
        ///
        /// # Examples
        /// Examples in `add_plan_works` , `add_plan_works2`
        #[ink(message)]
        pub fn add_plan(
            &mut self,
            durations: Vec<u64>,
            active_session_limits: Vec<u128>,
            prices: Vec<u128>,
            max_refund_permille_policies: Vec<u128>,
            plan_charastristics: Vec<Vec<String>>,
        ) {
            assert_eq!(durations.len(), active_session_limits.len(),"Wrong Number of Args");
            assert_eq!(prices.len(), active_session_limits.len(),"Wrong Number of Args");
            assert_eq!(
                max_refund_permille_policies.len(),
                active_session_limits.len(),
                "Wrong Number of Args"
            );
            assert_eq!(
                max_refund_permille_policies.len(),
                plan_charastristics.len(),
                "Wrong Number of Args"
            );

            let caller = self.env().caller();

            let provider = match self.providers.get_mut(&caller) {
                Some(x) => x,
                None => panic!("You should first register in the contract!"),
            };
            for i in 0..durations.len() {
                provider.plans.push(PlanConsts {
                    duration: durations[i],
                    active_session_limit: active_session_limits[i],
                    price: prices[i],
                    max_refund_permille_policy: max_refund_permille_policies[i],
                    disabled: false,
                });
                provider.plans_charastristics.push(plan_charastristics[i].clone());
            }
            for i in 0..durations.len() {
                self.env().emit_event(AddPlanEvent {
                    owner: caller,
                    duration: durations[i],
                    price: prices[i],
                });
            }
        }

        /// Editing previously created plans of the `caller`
        ///
        /// # Note
        ///
        /// This will not effect the users that subscribed prior to the edition of plan
        ///
        /// # Panics
        ///
        /// If `plan_index` is bigger than the length of `plans` of `provider`
        ///
        /// # Examples
        /// Examples of different situations in `edit_plan_works` and `edit_plan_works2` in `tests/test.rs`
        #[ink(message)]
        pub fn edit_plan(
            &mut self,
            plan_index: u128,
            duration: u64,
            active_session_limit: u128,
            price: u128,
            max_refund_permille_policies: u128,
            disabled: bool,
        ) {
            let number: usize = plan_index.try_into().unwrap();
            let caller = self.env().caller();

            let provider = match self.providers.get_mut(&caller) {
                Some(x) => x,
                None => panic!("You should first register in the contract!"),
            };

            let mut plan: &mut PlanConsts = match provider.plans.get_mut(number) {
                Some(x) => x,
                None => panic!("please select a valid plan")
            };

            plan.duration = duration;
            plan.active_session_limit = active_session_limit;
            plan.price = price;
            plan.max_refund_permille_policy = max_refund_permille_policies;
            plan.disabled = disabled;
        }

        #[ink(message)]
        pub fn add_charastristic_for_plan(
            &mut self,
            plan_index: u128,
            charastristics: Vec<String>,
        ) {
            let number: usize = plan_index.try_into().unwrap();
            let caller = self.env().caller();

            let provider = match self.providers.get_mut(&caller) {
                Some(x) => x,
                None => panic!("You should first register in the contract!"),
            };

            let plan: &mut Vec<String> = match provider.plans_charastristics.get_mut(number) {
                Some(x) => x,
                None => panic!("please select a valid plan")
            };

            for i in 0..charastristics.len() {
                plan.push(charastristics[i].clone());
            }

        }

        /// Disabling previously created plans of the `caller`
        ///
        /// # Note
        ///
        /// This will not effect the users that subscribed prior to the edition of plan
        ///
        /// # Panics
        ///
        /// If `plan_index` is bigger than the length of `plans` of `provider`
        ///
        /// # Examples
        /// Examples in `change_disable_works` in `tests/test.rs`
        #[ink(message)]
        pub fn change_disable(&mut self, plan_index: u128) {
            let caller = self.env().caller();            
            let number: usize = plan_index.try_into().unwrap();
            match self.providers.get_mut(&caller) {
                Some(provider) => {
                    assert!(
                        provider.plans.len() > plan_index.try_into().unwrap(),
                        "please select a valid plan"
                    );
                    provider.plans[number].disabled = !provider.plans[number].disabled
                },
                None => panic!("You should first register in the contract!")
            }
        }

        /// Subscribing to `plan_index` of the `provider_address` with `Sha2x256` hashed `pass` and `metadata`
        ///
        /// In this function, we will lock (`plan.max_refund_permille_policy` * `transferred_balance`) / 1000
        /// in the `Linked List` of the contract and will transfer the rest of paid money directly to provider
        ///
        /// # Note
        ///
        /// The `subs_crypt_pass_hash` will only be set if it's the first subscription of the `caller` to the `SubsCrypt` platform
        /// `caller` can not subscribe to same `plan_index` of the same `provider_address` but
        /// it can subscribe to different `plan_index` of same `provider_address` .
        /// This line of code checks that if you previously subscribed to `provider_address` and if it's the first time
        /// then `list_of_providers` will be updated.
        ///  `if !self.records.contains_key(&(caller, provider_address)) `
        ///
        ///
        /// # Panics
        /// If paid amount is not equal to `price` of the plan
        /// If plan is `disabled`
        /// If `caller` is already subscribed to plan
        /// If `provider` does not exist
        /// If `plan_index` is bigger than the length of `plans` of `provider_address`
        ///
        /// # Examples
        /// Examples in `subscribe_works` and `subscribe_works2` in `tests/test.rs`
        #[ink(message, payable)]
        pub fn subscribe(
            &mut self,
            provider_address: AccountId,
            plan_index: u128,
            pass: [u8; 32],
            username: String,
            charastristics_values_encrypted: Vec<String>,
        ) {
            let caller: AccountId = self.env().caller();
            let time: u64 = self.env().block_timestamp();

            assert!(
                !self.check_subscription(caller, provider_address, plan_index),
                "You are already subscribed to this plan!"
            );

            let provider = match self.providers.get(&provider_address) {
                Some(provider) => provider,
                None => panic!("Provider not existed in the contract!")
            };

            let index : usize = plan_index.try_into().unwrap();

            assert!(
                provider.plans.len() > plan_index.try_into().unwrap(),
                "Wrong plan index!"
            );

            let consts: PlanConsts = provider.plans[index];
            let plan_charastristics: Vec<String> = provider.plans_charastristics[index].clone();
            
            assert_eq!(charastristics_values_encrypted.len(), plan_charastristics.len(), "invalid charastristic values!");
            assert_eq!(consts.price, self.env().transferred_balance(), "You have to pay exact plan price");
            assert!(!consts.disabled, "Plan is currently disabled by provider");
            

            let plan_charastristics: Vec<String> = provider.plans_charastristics[index].clone();
            
            assert_eq!(charastristics_values_encrypted.len(), plan_charastristics.len(), "invalid charastristic values!");
            
            let mut address_has_not_username: bool = true;
            if self.address_to_username.contains_key(&caller) {
                address_has_not_username = false;
                match self.username_to_address.get(&username) {
                    Some(address) => assert_eq!(*address, caller, "this username is invalid!"),
                    None => {}
                }
            }
            
            let addr: &AccountId = &provider.money_address;
            // send money to money_address (1000 - plan.max_refund_permille_policy) / 1000;
            assert_eq!(self.transfer(
                    *addr,
                    consts.price * (1000 - consts.max_refund_permille_policy) / 1000
                ), Ok(())
            );

            if address_has_not_username {
                self.address_to_username.insert(caller, username.clone());
                self.username_to_address.insert(username, caller);
            }

            
            if !self.users.contains_key(&caller) {
                self.users.insert(
                    caller,
                    User {
                        list_of_providers: Vec::new(),
                        subs_crypt_pass_hash: pass,
                    },
                );
            }

            
            let subscription_record = SubscriptionRecord {
                provider: provider_address,
                plan: consts,
                plan_index,
                subscription_time: time,
                charastristics_values_encrypted,
                refunded: false,
            };

            if let Some(plan_record) = self.records.get_mut(&(caller, provider_address)) {
                self.plan_index_to_record_index.insert(
                    (caller, provider_address, plan_index),
                    plan_record.subscription_records.len().try_into().unwrap(),
                );

                plan_record.subscription_records.push(subscription_record);
            } else {
                self.users.get_mut(&caller).unwrap().list_of_providers.push(provider_address);

                let plan_record: PlanRecord = PlanRecord {
                    subscription_records: vec![subscription_record],                  
                    pass_hash: pass,
                };

                self.records.insert((caller, provider_address), plan_record);

                self.plan_index_to_record_index
                    .insert((caller, provider_address, plan_index), 0);
            }
            self.add_entry(
                provider_address,
                (time + consts.duration - self.start_time) / 86400,
                (self.env().transferred_balance() * consts.max_refund_permille_policy) / 1000,
            );

            self.env().emit_event(SubscribeEvent {
                provider: provider_address,
                plan_index: plan_index,
                subscription_time: time,
                duration: consts.duration,
            });
        }

        pub fn renew(&mut self,
            provider_address: AccountId,
            plan_index: u128,
            new_charastristics_values: Vec<String>,
        ) {
            let caller: AccountId = self.env().caller();

            if !self
                .plan_index_to_record_index
                .contains_key(&(caller, provider_address, plan_index))
            {
                panic!("You should have been subscribed to this plan for renew!");
            }

            let last_index : u128 = match self
            .plan_index_to_record_index
            .get(&(caller, provider_address, plan_index)) {
                Some(index) => *index,
                None => panic!("index is not valid!")
            };
            let number: usize = last_index.try_into().unwrap();
            let record = &self
                .records
                .get(&(caller, provider_address))
                .unwrap()
                .subscription_records[number];

            if record.plan_index != plan_index
                || record.refunded
                || record.plan.duration + record.subscription_time < self.env().block_timestamp()
            {
                panic!("You should have been subscribed to this plan for renew!");
            }

            let provider = match self.providers.get(&provider_address) {
                Some(provider) => provider,
                None => panic!("Provider not existed in the contract!")
            };
            let start_time: u64 = record.plan.duration + record.subscription_time;

            let index : usize = plan_index.try_into().unwrap();
            let consts: PlanConsts = provider.plans[index];


            assert_eq!(consts.price, self.env().transferred_balance(), "You have to pay exact plan price");
            assert!(!consts.disabled, "Plan is currently disabled by provider");

            let plan_charastristics: Vec<String> = provider.plans_charastristics[index].clone();
            assert_eq!(new_charastristics_values.len() + record.charastristics_values_encrypted.len(), plan_charastristics.len(), "invalid charastristic values!");
            
            let addr: &AccountId = &provider.money_address;
            // send money to money_address (1000 - plan.max_refund_permille_policy) / 1000;
            assert_eq!(self.transfer(
                    *addr,
                    consts.price * (1000 - consts.max_refund_permille_policy) / 1000
                ), Ok(())
            );
            let promised_amount = record.plan.price * record.plan.max_refund_permille_policy;
            assert_eq!(
                self.transfer(
                    provider.money_address,
                    promised_amount
                ),
                Ok(())
            );
            let passed_time = record.plan.duration + record.subscription_time - self.start_time;
            
            let charastristics_values: &mut Vec<String> =&mut record.charastristics_values_encrypted.clone();

            for i in 0..new_charastristics_values.len() {
                charastristics_values.push(new_charastristics_values[i].clone());
            }

            let subscription_record = SubscriptionRecord {
                provider: provider_address,
                plan: consts,
                plan_index,
                subscription_time: start_time,
                charastristics_values_encrypted: record.charastristics_values_encrypted.clone(),
                refunded: false,
            };
            
            self.remove_entry(provider_address, passed_time / 86400, promised_amount / 1000);

            let plan_record = self.records.get_mut(&(caller, provider_address)).unwrap();

            self.plan_index_to_record_index.insert(
                (caller, provider_address, plan_index),
                plan_record.subscription_records.len().try_into().unwrap(),
            );

            plan_record.subscription_records.push(subscription_record);
            
            self.add_entry(
                provider_address,
                (start_time + consts.duration - self.start_time) / 86400,
                (self.env().transferred_balance() * consts.max_refund_permille_policy) / 1000,
            );
            self.env().emit_event(SubscribeEvent {
                provider: provider_address,
                plan_index: plan_index,
                subscription_time: start_time,
                duration: consts.duration,
            });
   
        }

        /// Setting the `subs_crypt_pass_hash` of caller to `pass`
        ///
        /// # Note
        ///
        /// The `subs_crypt_pass_hash` will also be set in `subscribe` function in first subscription
        ///
        ///
        /// # Panics
        /// If `caller` does not exist in `users`
        #[ink(message)]
        pub fn set_user_subscrypt_pass(&mut self, pass: [u8; 32]) {
            match self.users.get_mut(&self.env().caller()) {
                Some(x) => x.subs_crypt_pass_hash = pass,
                None => panic!("User doesn't exist!")
            };
        }

        /// Setting the `subs_crypt_pass_hash_for_each_provider` of caller to `pass`
        ///
        /// # Panics
        /// If `caller` does not exist in `providers`
        #[ink(message)]
        pub fn subs_crypt_pass_hash_for_each_provider(&mut self, provider_address: AccountId, pass: [u8; 32]) {
            match  self.records.get_mut(&(self.env().caller(), provider_address)) {
                Some(x) => x.pass_hash = pass,
                None => panic!("User doesn't exist!")
            };
        }

        #[ink(message)]
        pub fn set_provider_subscrypt_pass(&mut self, pass: [u8; 32]) {
            match self.providers.get_mut(&self.env().caller()) {
                Some(x) => x.subs_crypt_pass_hash = pass,
                None => panic!("User doesn't exist!")
            };
        }

        /// This function is used when providers want to collect the locked money for refund policy
        ///
        /// In this function, we will unlock that money which was locked in `subscribe` function via the
        /// LinkedList mechanism, so providers can `withdraw` them when the due date passed.
        ///
        /// # Returns
        /// `paid` amount is returned
        ///
        /// # Panics
        /// If `provider` does not exist
        ///
        /// # Examples
        /// Examples in `withdraw_works` and `withdraw_works2` in `tests/test.rs`
        #[ink(message)]
        pub fn withdraw(&mut self) -> u128 {
            assert!(
                self.providers.contains_key(&self.env().caller()),
                "You are not a registered provider"
            );

            let caller: AccountId = self.env().caller();
            let t  = self.process(caller, self.env().block_timestamp() / 86400);
            if t.withdrawing_amount  > 0 {
                assert_eq!(self.transfer(caller, t.withdrawing_amount), Ok(()));
            }

            let linked_list: &mut LinkedList = &mut self
                    .providers
                    .get_mut(&caller)
                    .unwrap()
                    .payment_manager;
            linked_list.head = t.current_linked_list_head;
            linked_list.length -= t.reduced_length;
            t.withdrawing_amount
        }

        /// `users` can use this function to easily refund their subscription as the policy of that
        /// specific plan was set. The `users` will be paid back at most
        /// (`plan.max_refund_permille_policy` * `transferred_balance`) / 1000 and it will be linearly
        /// decreased as time passed and will get to 0. The `provider` will get 0 at least and will linearly
        /// get more if `user` refund later.
        ///
        /// # Returns
        /// `paid` amount is returned
        ///
        /// # Panics
        /// If `provider` does not exist
        ///
        /// # Examples
        /// Assume that `plan.max_refund_permille_policy` = 500 and `plan.price` = 100 the duration
        /// of the plan is a month(30 days month). if `user` refund in first half of the month, then the user will
        /// be paid 50. if `user` refund in day 20th of month then `user` will be paid 33.33 and `provider`
        /// will be paid 16.66.
        /// Other Examples in `refund_works` and `refund_works2` in `tests/test.rs`
        #[ink(message)]
        pub fn refund(&mut self, provider_address: AccountId, plan_index: u128) -> u128 {
            let caller: AccountId = self.env().caller();
            let time: u64 = self.env().block_timestamp();
            assert!(
                self.check_subscription(caller, provider_address, plan_index),
                "You are not in this plan or already refunded"
            );

            let last_index = match self
            .plan_index_to_record_index
            .get(&(caller, provider_address, plan_index)) {
                Some(index) => index,
                None => panic!("index is not valid!")
            };

            let number: usize = (*last_index).try_into().unwrap();
            let record: &SubscriptionRecord = self
                .records
                .get(&(caller, provider_address))
                .unwrap()
                .subscription_records
                .get(number)
                .unwrap();



            assert!(time - record.subscription_time < record.plan.duration);

            let promised_amount = record.plan.price * record.plan.max_refund_permille_policy;
            let price : u64 = (record.plan.price * 1000).try_into().unwrap();
            let used : u64 = price * (time - record.subscription_time) / record.plan.duration;
            let mut customer_portion_locked_money : u128 = (price - used).try_into().unwrap();

            if customer_portion_locked_money > promised_amount {
                // in this case the customer wants to refund very early so he want to get
                // more than the amount of refund policy, so we can only give back just
                // max_refund_permille_policy of his/her subscription. Whole locked money will go directly to
                // account of the customer

                customer_portion_locked_money = promised_amount;
            } else {
                // in this case the customer wants to refund, but he/she used most of his subscription time
                // and now he/she will get portion of locked money, and the provider will get the rest of money
                
                let provider_portion_locked_money = (promised_amount - customer_portion_locked_money) / 1000;
                assert_eq!(
                    self.transfer(
                        self.providers.get(&provider_address).unwrap().money_address,
                        provider_portion_locked_money
                    ),
                    Ok(())
                );
            }
            assert_eq!(self.transfer(caller, customer_portion_locked_money / 1000), Ok(()));

            let passed_time = record.plan.duration + record.subscription_time - self.start_time;
            self.remove_entry(provider_address, passed_time / 86400, promised_amount / 1000);
            self.records
                .get_mut(&(caller, provider_address))
                .unwrap()
                .subscription_records
                .get_mut(number)
                .unwrap()
                .refunded = true;
            customer_portion_locked_money
        }

        /// This function indicate if `user` can authenticate with given `token` and `pass_phrase`
        /// # Note
        /// `user` are encouraged to have different `token` and `pass_phrase` for each provider
        ///
        /// # Returns
        /// `bool` is returned which shows the correctness of auth
        ///
        /// # Example
        /// Examples in `check_auth_works` in `tests/test.rs`
        #[ink(message)]
        pub fn check_auth(
            &self,
            user: AccountId,
            provider: AccountId,
            pass_phrase: String,
        ) -> bool {
            return match self.records.get(&(user, provider)) {
                Some(record) => {
                    let encoded = self.env().hash_encoded::<Sha2x256, _>(&pass_phrase);
                    return encoded == record.pass_hash
                },
                None => false
            }
        }

        /// This function indicate if `username` can authenticate with given `pass_phrase`
        /// # Note
        /// `user` are encouraged to have different `token` and `pass_phrase` for each provider
        ///
        /// # Returns
        /// `bool` is returned which shows the correctness of auth
        ///
        /// # Example
        /// Examples in `check_auth_works` in `tests/test.rs`
        #[ink(message)]
        pub fn check_auth_with_username(
            &self,
            username: String,
            provider: AccountId,
            pass_phrase: String,
        ) -> bool {
            let user = match self.username_to_address.get(&username) {
                Some(name) => *name,
                None => panic!("this username is invalid!")
            };
            self.check_auth(user, provider, pass_phrase)
        }

        #[ink(message)]
        pub fn provider_check_auth(
            &self,
            provider: AccountId,
            pass_phrase: String,
        ) -> bool {
            return match self.providers.get(&provider) {
                Some(provider) => {
                    let encoded = self.env().hash_encoded::<Sha2x256, _>(&pass_phrase);
                    return encoded == provider.subs_crypt_pass_hash
                },
                None => false
            }
        }

        #[ink(message)]
        pub fn provider_check_auth_with_username(
            &self,
            username: String,
            pass_phrase: String,
        ) -> bool {
            let address = match self.username_to_address.get(&username) {
                Some(name) => *name,
                None => panic!("this username is invalid!")
            };
            self.provider_check_auth(address, pass_phrase)
        }

        
        #[ink(message)]
        pub fn user_check_auth(
            &self,
            user: AccountId,
            pass_phrase: String,
        ) -> bool {
            return match self.users.get(&user) {
                Some(user) => {
                    let encoded = self.env().hash_encoded::<Sha2x256, _>(&pass_phrase);
                    return encoded == user.subs_crypt_pass_hash
                },
                None => false
            }
        }

        #[ink(message)]
        pub fn user_check_auth_with_username(
            &self,
            username: String,
            pass_phrase: String,
        ) -> bool {
            let address = match self.username_to_address.get(&username) {
                Some(name) => *name,
                None => panic!("this username is invalid!")
            };
            self.user_check_auth(address, pass_phrase)
        }


        #[ink(message)]
        pub fn is_username_available(
            &self,
            username: String,
        ) -> bool {
            !self.username_to_address.contains_key(&username)
        }


        pub fn get_username_by_address(
            &self,
            address: AccountId,
        ) -> String {
            match self.address_to_username.get(&address) {
                Some(username) => username.to_string(),
                None => panic!("this address has not a valid associated username!")
            }
        }


        /// `user` can use this function to retrieve her whole subscription history to
        /// different providers.
        /// # Note
        /// `user` has to provide their main `token` and `phrase` which will be used in
        /// SubsCrypt dashboard
        ///
        /// # Returns
        /// `Vec<SubscriptionRecord>` is returned which is a vector of `SubscriptionRecord` struct
        ///
        /// # Example
        /// Examples in `retrieve_whole_data_with_password_works` in `tests/test.rs`
        #[ink(message)]
        pub fn retrieve_whole_data_with_username(
            &self,
            username: String,
            phrase: String,
        ) -> Vec<SubscriptionRecord> {
            let user = match self.username_to_address.get(&username) {
                Some(name) => *name,
                None => panic!("this username is invalid!")
            };
            let encoded = self.env().hash_encoded::<Sha2x256, _>(&phrase);
            assert_eq!(
                encoded,
                self.users.get(&user).unwrap().subs_crypt_pass_hash,
                "Wrong auth"
            );
            self.retrieve_whole_data(user)
        }

        /// `user` can use this function to retrieve her whole subscription history to
        /// different providers with their wallet.
        ///
        /// # Returns
        /// `Vec<SubscriptionRecord>` is returned which is a vector of `SubscriptionRecord` struct
        ///
        /// # Example
        /// Examples in `retrieve_whole_data_with_wallet_works` in `tests/test.rs`
        #[ink(message)]
        pub fn retrieve_whole_data_with_wallet(&self) -> Vec<SubscriptionRecord> {
            let caller: AccountId = self.env().caller();
            self.retrieve_whole_data(caller)
        }

        /// `user` can use this function to retrieve her subscriptions history for a specific
        /// provider.
        ///
        /// # Note
        /// `user` has to provide their `token` and `phrase` for that provider.
        ///
        /// # Returns
        /// `Vec<SubscriptionRecord>` is returned which is a vector of `SubscriptionRecord` struct
        ///
        /// # Example
        /// Examples in `retrieve_data_with_password_works` in `tests/test.rs`
        #[ink(message)]
        pub fn retrieve_data_with_username(
            &self,
            username: String,
            provider_address: AccountId,
            phrase: String,
        ) -> Vec<SubscriptionRecord> {
            let user = match self.username_to_address.get(&username) {
                Some(name) => *name,
                None => panic!("this username is invalid!")
            };
            let encoded = self.env().hash_encoded::<Sha2x256, _>(&phrase);
            assert_eq!(
                encoded,
                self.records
                    .get(&(user, provider_address))
                    .unwrap()
                    .pass_hash,
                "Wrong auth"
            );
            self.retrieve_data(user, provider_address)
        }

        /// `user` can use this function to retrieve her subscriptions history for a specific
        /// provider with her wallet.
        ///
        /// # Returns
        /// `Vec<SubscriptionRecord>` is returned which is a vector of `SubscriptionRecord` struct
        ///
        /// # Example
        /// Examples in `retrieve_data_with_password_works` in `tests/test.rs`
        #[ink(message)]
        pub fn retrieve_data_with_wallet(
            &self,
            provider_address: AccountId,
        ) -> Vec<SubscriptionRecord> {
            let caller: AccountId = self.env().caller();
            self.retrieve_data(caller, provider_address)
        }

        #[ink(message)]
        pub fn get_plan_data(&self, provider_address: AccountId, plan_index: u128) ->  PlanConsts {
            let number: usize = plan_index.try_into().unwrap();
            match match self
            .providers
            .get(&provider_address) {
                Some(provider) => provider,
                None => panic!("index is not valid!")
            }.plans.get(number) {
                Some(x) => *x,
                None => panic!("please select a valid plan")
            }
        }

        #[ink(message)]
        pub fn get_plan_charastristics(&self, provider_address: AccountId, plan_index: u128) ->  Vec<String> {
            let number: usize = plan_index.try_into().unwrap();
            match match self
            .providers
            .get(&provider_address) {
                Some(provider) => provider,
                None => panic!("index is not valid!")
            }.plans_charastristics.get(number) {
                Some(x) => x.clone(),
                None => panic!("please select a valid plan")
            }
        }


        /// This function can be called to check if `user` has a valid subscription to the
        /// specific `plan_index` of `provider`.
        ///
        /// # Note
        /// if `user` refunded or her subscription is expired then this function will return false
        ///
        /// # Returns
        /// `bool` which means if `user` is subsribed or not
        ///
        /// # Example
        /// Examples in `check_subscription_works` in `tests/test.rs`
        #[ink(message)]
        pub fn check_subscription(
            &self,
            user: AccountId,
            provider_address: AccountId,
            plan_index: u128,
        ) -> bool {
            if !self
                .plan_index_to_record_index
                .contains_key(&(user, provider_address, plan_index))
            {
                return false;
            }
            let last_index: u128 = *self
                .plan_index_to_record_index
                .get(&(user, provider_address, plan_index))
                .unwrap();
            let number: usize = last_index.try_into().unwrap();
            let record: &SubscriptionRecord = &self
                .records
                .get(&(user, provider_address))
                .unwrap()
                .subscription_records[number];
            if record.plan_index != plan_index
                || record.refunded
                || record.plan.duration + record.subscription_time < self.env().block_timestamp()
            {
                return false;
            }
            true
        }

        #[ink(message)]
        pub fn check_subscription_with_username(
            &self,
            username: String,
            provider_address: AccountId,
            plan_index: u128,
        ) -> bool {
            let user = match self.username_to_address.get(&username) {
                Some(name) => *name,
                None => panic!("this username is invalid!")
            };
            self.check_subscription(user, provider_address, plan_index)
        }

        fn retrieve_whole_data(&self, caller: AccountId) -> Vec<SubscriptionRecord> {
            assert!(self.users.contains_key(&caller));
            let mut data: Vec<SubscriptionRecord> = Vec::new();
            let user: &User = self.users.get(&caller).unwrap();
            for i in 0..user.list_of_providers.len() {
                data.append(&mut self.retrieve_data(caller, user.list_of_providers[i]));
            }
            data
        }

        fn retrieve_data(
            &self,
            caller: AccountId,
            provider_address: AccountId,
        ) -> Vec<SubscriptionRecord> {
            assert!(self.users.contains_key(&caller));
            assert!(self.records.contains_key(&(caller, provider_address)));
            let mut data: Vec<SubscriptionRecord> = Vec::new();

            let plan_records: &PlanRecord = self.records.get(&(caller, provider_address)).unwrap();
            for i in 0..plan_records.subscription_records.len() {
                let k = SubscriptionRecord {
                    provider: plan_records.subscription_records[i].provider,
                    plan: plan_records.subscription_records[i].plan,
                    plan_index: plan_records.subscription_records[i].plan_index,
                    subscription_time: plan_records.subscription_records[i].subscription_time,
                    charastristics_values_encrypted: plan_records.subscription_records[i]
                        .charastristics_values_encrypted
                        .clone(),
                    
                    refunded: plan_records.subscription_records[i].refunded,
                };
                data.push(k);
            }
            data
        }

        fn transfer(&self, addr: AccountId, amount: u128) -> Result<(), Error> {
            self.env().transfer(addr, amount).map_err(|err| match err {
                Error::BelowSubsistenceThreshold => Error::BelowSubsistenceThreshold,
                _ => Error::TransferFailed,
            })
        }

        /// add_entry : add a payment entry to provider payment management linked list
        /// # arguments:
        /// * provider_address
        /// * day_id : the calculation formula is : (finish date - contract start date) / 86400
        /// * amount : money amount
        fn add_entry(&mut self, provider_address: AccountId, day_id: u64, amount: u128) {
            let linked_list: &mut LinkedList = &mut self
                .providers
                .get_mut(&provider_address)
                .unwrap()
                .payment_manager;
            if linked_list.length == 0 {
                let object = DailyLockedAmount {
                    amount,
                    next_day: day_id,
                };
                linked_list.head = day_id;
                self.daily_locked_amounts
                    .insert((provider_address, day_id), object);
                linked_list.back = day_id;
                linked_list.length += 1;
            } else if day_id < linked_list.head {
                let object = DailyLockedAmount {
                    amount,
                    next_day: linked_list.head,
                };
                linked_list.head = day_id;
                self.daily_locked_amounts
                    .insert((provider_address, day_id), object);
                linked_list.length += 1;
            } else if day_id > linked_list.back {
                self.daily_locked_amounts
                    .get_mut(&(provider_address, linked_list.back))
                    .unwrap()
                    .next_day = day_id;
                let object = DailyLockedAmount {
                    amount,
                    next_day: day_id,
                };
                linked_list.back = day_id;
                self.daily_locked_amounts
                    .insert((provider_address, day_id), object);
                linked_list.length += 1;
            } else {
                let mut cur_id: u64 = linked_list.head;
                loop {
                    if day_id == cur_id {
                        self.daily_locked_amounts
                            .get_mut(&(provider_address, day_id))
                            .unwrap()
                            .amount += amount;
                        break;
                    } else if day_id
                        < self
                            .daily_locked_amounts
                            .get(&(provider_address, cur_id))
                            .unwrap()
                            .next_day
                    {
                        let object = DailyLockedAmount {
                            amount,
                            next_day: self
                                .daily_locked_amounts
                                .get(&(provider_address, cur_id))
                                .unwrap()
                                .next_day,
                        };
                        self.daily_locked_amounts
                            .get_mut(&(provider_address, cur_id))
                            .unwrap()
                            .next_day = day_id;
                        self.daily_locked_amounts
                            .insert((provider_address, day_id), object);
                        linked_list.length += 1;
                        break;
                    }
                    cur_id = self
                        .daily_locked_amounts
                        .get(&(provider_address, cur_id))
                        .unwrap()
                        .next_day;
                    if cur_id == linked_list.back {
                        break;
                    }
                }
            }
        }

        /// remove_entry : when a user refunds this function removes its related entry
        /// # arguments:
        /// * provider_address
        /// * day_id : the calculation formula is : (finish date - contract start date) / 86400
        /// * amount
        fn remove_entry(&mut self, provider_address: AccountId, day_id: u64, amount: u128) {
            self.daily_locked_amounts
                .get_mut(&(provider_address, day_id))
                .unwrap()
                .amount -= amount;
        }

        /// process : when providers withdraw this function calculates the amount of money
        /// # arguments:
        /// * provider_address
        /// * day_id : the calculation formula is : (finish date - contract start date) / 86400
        pub fn process(&mut self, provider_address: AccountId, day_id: u64) -> ProcessReturningData {
            let linked_list: &mut LinkedList = &mut self
                .providers
                .get_mut(&provider_address)
                .unwrap()
                .payment_manager;
            let mut sum: u128 = 0;
            let mut reduced_length = 0;
            let mut cur_id: u64 = linked_list.head;
            while day_id >= cur_id {
                sum += self
                    .daily_locked_amounts
                    .get(&(provider_address, cur_id))
                    .unwrap()
                    .amount;
                cur_id = self
                    .daily_locked_amounts
                    .get(&(provider_address, cur_id))
                    .unwrap()
                    .next_day;
                reduced_length += 1;
                if cur_id == linked_list.back {
                    break;
                }
            }

            ProcessReturningData{
                withdrawing_amount: sum,
                current_linked_list_head: cur_id,
                reduced_length: reduced_length
            }
        }
    }

    

    impl Default for LinkedList {
        fn default() -> Self {
            Self::new()
        }
    }

    impl LinkedList {
        pub fn new() -> Self {
            LinkedList::default()
        }
        pub fn default() -> Self {
            Self {
                back: 0,
                head: 0,
                length: 0,
            }
        }
    }
}
