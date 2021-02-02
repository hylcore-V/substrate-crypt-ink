#![cfg_attr(not(feature = "std"), no_std)]
use hex_literal::hex;
use sha2::{Sha256, Sha512, Digest};
use ink_lang as ink;
use ink_prelude::vec::Vec;
use ink_storage::collections::HashMap;

#[ink::contract]
mod subscrypt {
    use ink_storage::collections;
    use ink_storage::collections::HashMap;
    use ink_env::AccountId;

    struct SubscriptionRecord {
        provider: AccountId,
        plan: PlanConsts,
        plan_index: u256,
        subscription_time: u256,
        meta_data_encrypted: string,
        //encrypted Data with public key of provider
        refunded: bool,
    }

    struct PlanRecord {
        plan_index_to_record_index: HashMap<u256, u256>,
        subscription_records: vec<SubscriptionRecord>,
        pass_hash: string,
    }

    struct PlanConsts {
        duration: u256,
        active_session_limit: u256,
        price: u256,
        max_refund_percent_policy: u256,
        disabled: bool,
    }

    struct Provider {
        plans: vec<PlanConsts>,
        money_address: AccountId,
        payment_manager: linked_list,
    }

    struct User {
        records: HashMap<AccountId, PlanRecord>,
        list_of_providers: vec<AccountId>,
        joined_time: u256,
        subs_crypt_pass_hash: String,
    }

    struct LinkedList {
        head: u256,
        back: u256,
        objects: HashMap<u256, Object>,
        length: u256,
    }

    struct Object {
        number: u256,
        next_day: u256,
    }

    #[ink(storage)]
    pub struct Subscrypt {
        start_time: u64,
        provider_register_fee: u64,
        providers: HashMap<AccountId, Provider>,
        users: HashMap<AccountId, User>,
    }

    impl Subscrypt {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                start_time: Self.env().block_timestamp(),
                provider_register_fee: 100,
                providers: ink_storage::collections::HashMap::new(),
                users: ink_storage::collections::HashMap::new(),
            }
        }

        #[ink(constructor)]
        pub fn default() -> Self {
            Self {
                start_time: 0,
                provider_register_fee: 0,
                providers: Default::default(),
                users: Default::default(),
            }
        }

        #[ink(message, payable)]
        pub fn provider_register(&mut self, durations: vec<u256>, active_session_limits: vec<u256>, prices: vec<u256>, max_refund_percent_policies: vec<u256>, address: AccountId) {
            let caller = self.env().caller();
            assert!(self.env().transferred_balance() => self.provider_register_fee, "You have to pay a minimum amount to register in the contract!");
            assert!(!self.providers.contains_key(caller), "You can not register again in the contract!");

            let provider = Provider {
                plans: vec::new(),
                money_address: address,
                payment_manager: LinkedList::new(),
            };

            self.providers.insert(caller, provider);
            for i in 0..durations.length {
                let cons = PlanConsts {
                    duration: durations[i],
                    active_session_limit: active_session_limits[i],
                    price: prices[i],
                    max_refund_percent_policy: max_refund_percent_policies[i],
                    disabled: false,
                };
                provider.plans.insert(cons);
            }
        }

        #[ink(message)]
        pub fn add_plan(&mut self, durations: vec<u256>, active_session_limits: vec<u256>, prices: vec<u256>, max_refund_percent_policies: vec<u256>) {
            let caller = self.env().caller();
            assert!(self.providers.contains_key(caller), "You should first register in the contract!");
            let provider = self.providers.get(caller).unwrap();
            for i in 0..durations.length {
                let cons = PlanConsts {
                    duration: durations[i],
                    active_session_limit: active_session_limits[i],
                    price: prices[i],
                    max_refund_percent_policy: max_refund_percent_policies[i],
                    disabled: false,
                };
                provider.plans.insert(cons);
            }
        }

        #[ink(message)]
        pub fn edit_plan(&mut self, plan_index: u256, duration: u256, active_session_limit: u256, price: u256, max_refund_percent_policy: u256) {
            let caller = self.env().caller();
            assert!(self.providers.get(&caller).unwrap().plans.contains_key(&plan_index), "please select a valid plan");

            let x = self.providers.get(&caller).unwrap().plans.get(&plan_index).unwrap();
            x.duration = duration;
            x.active_session_limit = active_session_limit;
            x.price = price;
            x.max_refund_percent_policy = max_refund_percent_policy;
            x.disabled = disabled;
        }

        #[ink(message)]
        pub fn change_disable(&mut self, plan_index: u64) {
            let caller = self.env().caller();
            assert!(self.providers.get(&caller).plans.contains_key(&plan_index), "please select a valid plan");
            let x = self.providers.get(&caller).plans.get(&plan_index).unwrap().disabled;
            self.providers.get(&caller).plans.get(&plan_index).disabled = !x;
        }

        #[ink(message, payable)]
        pub fn subscribe(&mut self, provider_address: AccountId, plan_index: u256, pass: string, metadata: string) {
            let caller: AccountId = self.env().caller();
            if !self.users.contains_key(&caller) {
                self.users.insert(caller, User {
                    records: HashMap::new(),
                    list_of_providers: vec::new(),
                    joined_time: Self.env().block_timestamp(),
                    subs_crypt_pass_hash: "".to_string(),
                });
            }

            let mut user: User = match self.users.get(&caller) { some => k };

            assert!(self.providers.contains_key(&provider_address), "Provider not existed in the contract!");
            assert!(self.providers.get(&provider_address).unwrap().plans.length > plan_index, "Wrong plan index!");
            let consts: PlanConsts = self.providers.get(&provider_address).unwrap().plans.get(&plan_index).unwrap();

            assert_eq!(consts.price, self.env().transferred_balance(), "You have to pay exact plan price");
            assert!(!consts.disabled, "Plan is currently disabled by provider");
            assert!(!check_subscription(self, caller, provider_address, plan_index: u256), "You are already subscribed to this plan!");

            if !user.records.contains_key(&provider_address) {
                user.list_of_providers.insert(provider_address);
            }
            let mut plan_record: PlanRecord = match user.records.get(&provider_address) { some => k };
            plan_record.plan_index_to_record_index.insert(plan_index, user.records[provider_address].subscription_records.len());

            let record: SubscriptionRecord = SubscriptionRecord {
                provider: provider_address,
                plan: consts.clone(),
                plan_index,
                subscription_time: Self.env().block_timestamp(),
                meta_data_encrypted: metadata,
                refunded: false,
            };
            plan_record.subscription_records.insert(record);

            plan_record.pass_hash = pass;

            addr: AccountId = self.providers.get(&provider_address).unwrap().money_address;
            // send money to money_address (1000 - plan.max_refund_percent_policy) / 1000;
            transfer(&self, self.env().caller(), consts.price(1000 - plan.max_refund_percent_policy) / 1000);

            self.providers.get(&provider_address).unwrap().payment_manager.addEntry((Self.env().block_timestamp() + consts.duration - &self.start_time) / 86400, (self.env().transferred_balance() * consts.max_refund_percent_policy) / 1000);
        }


        #[ink(message)]
        pub fn set_subscrypt_pass(&mut self, pass: string) {
            assert!(self.users.contains_key(self.env().caller()));
            self.users.get(&self.env().caller()).unwrap().subs_crypt_pass_hash = pass;
        }

        #[ink(message)]
        pub fn refund(&mut self, provider_address: u256, plan_index: u256) {
            let caller: AccountId = self.env().caller();
            assert!(self.check_subscription(caller, provider_address, plan_index));
            last_index: u256 = self.users.get(caller).unwrap().records.get(provider_address).unwrap().planIndexToRecordIndex.get(plan_index).unwrap();
            record: SubscriptionRecord = self.users.get(caller).unwrap().records.get(provider_address).unwrap().subscriptionRecords.get(last_index).unwrap();
            time_percent: u256 = (self.env().block_timestamp() - record.subscription_time) * 1000 / (record.plan.duration);
            if 1000 - time_percent > record.plan.max_refund_percent_policy {
                time_percent = record.plan.max_refund_percent_policy;
            } else {
                time_percent = 1000 - time_percent;
            }
            transfer_value: u256 = time_percent * record.plan.price / 1000;
            record.refunded = true;
            transfer(self, caller, transfer_value);
            if time_percent < record.plan.max_refund_percent_policy {
                refunded_amount: u256 = (record.plan.max_refund_percent_policy - time_percent) * record.plan.price / 1000;
                transfer(self, self.providers.get(provider_address).unwrap().money_address, transfer_value);
            }

            self.providers.get(provider_address).unwrap().payment_manager.remove_entry((record.plan.duration + record.subscription_time - &self.start_time) / 86400, record.plan.price * record.plan.maxRefundPercentPolicy);
        }

        #[ink(message)]
        pub fn withdraw(&mut self) {
            assert!(self.providers.contains_key(self.env().caller()), "You are not a registered provider");
            let caller: AccountId = self.env().caller();
            paid: u256 = self.providers.get(caller).unwrap().payment_manager.process();
            if paid > 0 {
                transfer(&self, caller, paid);
            }
            return paid;
        }

        #[ink(message)]
        pub fn check_auth(&self, user: AccountId, provider: AccountId, token: bytes32, pass_phrase: bytes32) {
            let mut hasher = Sha256::new();
            hasher.update(b"hello world");
            let result = hasher.finalize();
        }

        #[ink(message)]
        pub fn retrieve_whole_data_with_password(&self) {
            self.my_value_or_zero(&self.env().caller())
        }

        #[ink(message)]
        pub fn retrieve_whole_data_with_wallet(&self) {
            self.my_value_or_zero(&self.env().caller())
        }

        #[ink(message)]
        pub fn retrieve_data_with_password(&self) {
            self.my_value_or_zero(&self.env().caller())
        }

        #[ink(message)]
        pub fn retrieve_data_with_wallet(&self) {
            self.my_value_or_zero(&self.env().caller())
        }


        fn check_subscription(str: &Subscrypt, caller: AccountId, provider_address: AccountId, plan_index: u256) {
            unimplemented!()
        }

        fn transfer(&self, addr: AccountId, amount: u256) {
            self.env()
                .transfer(addr, amount)
                .map_err(|err| {
                    match err {
                        ink_env::Error::BelowSubsistenceThreshold => {
                            Error::BelowSubsistenceThreshold
                        }
                        _ => Error::TransferFailed,
                    }
                });
        }
    }


    impl LinkedList {
        pub fn new() -> Self {
            Self {
                back: 0,
                head: 0,
                objects: collections::HashMap::new(),
                length: 0,
            }
        }

        pub fn add_entry(&mut self, day_id: u256, amount: u256) {
            if self.length == 0 {
                let object = Object { number: amount, next_day: day_id };
                self.head = day_id;
                self.objects.insert(day_id, object);
                self.back = day_id;
                self.length = self.length + 1;
            } else if day_id < self.head {
                let object = Object { number: amount, next_day: day_id };
                self.head = dayID;
                self.objects.insert(day_id, object);
                self.length = self.length + 1;
            } else if day_id > self.back {
                self.objects.get(day_id).nextDay = dayID;
                let object = Object { number: amount, next_day: day_id };
                self.back = dayID;
                self.objects.insert(day_id, object);
                self.length = self.length + 1;
            } else {
                let mut finish: bool = false;
                let mut cur_id: u256 = self.head;
                while !finish {
                    if day_id == cur_id {
                        self.objects.get(day_id).number += amount;
                        break;
                    } else if day_id < self.objects.get(cur_id).next_day {
                        let object = Object { number: amount, next_day: self.objects.get(cur_id).next_day };
                        self.objects.get(cur_id).next_day = day_id;
                        self.objects.insert(day_id, object);
                        self.length = self.length + 1;
                        break;
                    }
                    cur_id = self.objects.get(cur_id).next_day;
                    if cur_id == self.back {
                        break;
                    }
                }
            }
        }

        pub fn remove_entry(&mut self, day_id: u256, amount: u256) {
            self.objects.get(day_id).number -= amount;
        }

        pub fn process(&mut self, day_id: u256) -> u256 {
            let sum: u256 = 0;
            let cur_id: u256 = self.head;
            while day_id >= cur_id {
                sum += self.objects.get(cur_id).number;
                cur_id = self.objects.get(cur_id).next_day;
                self.length -= 1;
                if cur_id == back {
                    break;
                }
            }
            self.head = cur_id;
            return sum;
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use ink_lang as ink;

        #[test]
        fn default_works() {
            let contract = Incrementer::default();
            assert_eq!(contract.get(), 0);
        }

        #[test]
        fn it_works() {
            let mut contract = Incrementer::new(42);
            assert_eq!(contract.get(), 42);
            contract.inc(5);
            assert_eq!(contract.get(), 47);
            contract.inc(-50);
            assert_eq!(contract.get(), -3);
        }

        #[ink::test]
        fn my_value_works() {
            let mut contract = Incrementer::new(11);
            assert_eq!(contract.get(), 11);
            assert_eq!(contract.get_mine(), 0);
            contract.inc_mine(5);
            assert_eq!(contract.get_mine(), 5);
            contract.inc_mine(10);
            assert_eq!(contract.get_mine(), 15);
        }
    }
}
