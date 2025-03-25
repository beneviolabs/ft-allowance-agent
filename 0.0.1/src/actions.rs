use near_sdk::{near, AccountId, Gas, NearToken};

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct NearAction {
    pub method_name: String,
    pub contract_id: AccountId,
    pub gas_attached: Gas,
    pub deposit_attached: NearToken,
}

impl NearAction {
    pub fn is_allowed(&self) {
        let allowed_contracts = ["wrap.near", "intents.near", "wrap.testnet"];
        let allowed_methods = [ "add_public_key", "ft_transfer_call", "near_deposit", "sign_intent"];
        if !allowed_contracts.contains(&self.contract_id.as_str()) {
            panic!(
                "{} is not allowed. Only wrap.near, wrap.testnet, and intents.near are permitted",
                self.contract_id
            );
        }
        if !allowed_methods.contains(&self.method_name.as_str()) {
            panic!("Method {} is restricted", self.method_name);
        }
    }
}
