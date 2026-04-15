#![cfg(test)]

use super::*;
use soroban_sdk::{
	testutils::{Address as _, Ledger},
	token, vec, Address, Env,
};

fn setup_token(env: &Env) -> (Address, token::Client<'_>, token::StellarAssetClient<'_>) {
	let admin = Address::generate(env);
	let token_address = env.register_stellar_asset_contract_v2(admin).address();
	let token_client = token::Client::new(env, &token_address);
	let token_admin_client = token::StellarAssetClient::new(env, &token_address);
	(token_address, token_client, token_admin_client)
}

#[test]
fn split_bill_paid_on_time_then_settled() {
	let env = Env::default();
	env.mock_all_auths();
 
	env.ledger().with_mut(|ledger| {
		ledger.timestamp = 1_000;
	});

	let creator = Address::generate(&env);
	let member_a = Address::generate(&env);
	let member_b = Address::generate(&env);

	let (token_address, token_client, token_admin_client) = setup_token(&env);
	token_admin_client.mint(&member_a, &500);
	token_admin_client.mint(&member_b, &500);

	let contract_id = env.register(SplitBillContract, ());
	let client = SplitBillContractClient::new(&env, &contract_id);

	let bill_id = client.create_bill(
		&creator,
		&token_address,
		&vec![&env, member_a.clone(), member_b.clone()],
		&vec![&env, 100_i128, 200_i128],
		&1_100_u64,
		&10_u32,
	);

	assert_eq!(client.get_member_due(&bill_id, &member_a), 100);
	assert_eq!(client.get_member_due(&bill_id, &member_b), 200);

	client.pay_share(&bill_id, &member_a);
	client.pay_share(&bill_id, &member_b);

	let bill = client.get_bill(&bill_id);
	assert!(bill.settled);
	assert_eq!(bill.total_collected, 300);
	assert_eq!(token_client.balance(&creator), 300);
}

#[test]
fn split_bill_late_payment_adds_penalty() {
	let env = Env::default();
	env.mock_all_auths();

	env.ledger().with_mut(|ledger| {
		ledger.timestamp = 2_000;
	});

	let creator = Address::generate(&env);
	let member_a = Address::generate(&env);
	let member_b = Address::generate(&env);

	let (token_address, token_client, token_admin_client) = setup_token(&env);
	token_admin_client.mint(&member_a, &1_000);
	token_admin_client.mint(&member_b, &1_000);

	let contract_id = env.register(SplitBillContract, ());
	let client = SplitBillContractClient::new(&env, &contract_id);

	let bill_id = client.create_bill(
		&creator,
		&token_address,
		&vec![&env, member_a.clone(), member_b.clone()],
		&vec![&env, 100_i128, 100_i128],
		&2_050_u64,
		&20_u32,
	);

	client.pay_share(&bill_id, &member_a);

	env.ledger().with_mut(|ledger| {
		ledger.timestamp = 2_200;
	});

	assert_eq!(client.get_member_due(&bill_id, &member_b), 120);
	client.pay_share(&bill_id, &member_b);

	let bill = client.get_bill(&bill_id);
	assert!(bill.settled);
	assert_eq!(bill.total_collected, 220);

	let second_member_state = bill.members.get(1).unwrap();
	assert!(second_member_state.late);
	assert_eq!(second_member_state.paid_amount, 120);
	assert_eq!(token_client.balance(&creator), 220);
}

#[test]
fn split_bill_supports_multiple_active_bills() {
	let env = Env::default();
	env.mock_all_auths();

	env.ledger().with_mut(|ledger| {
		ledger.timestamp = 3_000;
	});

	let creator = Address::generate(&env);
	let member_a = Address::generate(&env);
	let member_b = Address::generate(&env);
	let member_c = Address::generate(&env);

	let (token_address, token_client, token_admin_client) = setup_token(&env);
	token_admin_client.mint(&member_a, &1_000);
	token_admin_client.mint(&member_b, &1_000);
	token_admin_client.mint(&member_c, &1_000);

	let contract_id = env.register(SplitBillContract, ());
	let client = SplitBillContractClient::new(&env, &contract_id);

	let bill_one = client.create_bill(
		&creator,
		&token_address,
		&vec![&env, member_a.clone(), member_b.clone()],
		&vec![&env, 100_i128, 200_i128],
		&3_100_u64,
		&10_u32,
	);

	let bill_two = client.create_bill(
		&creator,
		&token_address,
		&vec![&env, member_c.clone()],
		&vec![&env, 300_i128],
		&3_150_u64,
		&15_u32,
	);

	assert_ne!(bill_one, bill_two);
	assert_eq!(client.get_member_due(&bill_one, &member_a), 100);
	assert_eq!(client.get_member_due(&bill_two, &member_c), 300);

	client.pay_share(&bill_two, &member_c);
	let settled_bill_two = client.get_bill(&bill_two);
	assert!(settled_bill_two.settled);

	let unsettled_bill_one = client.get_bill(&bill_one);
	assert!(!unsettled_bill_one.settled);

	client.pay_share(&bill_one, &member_a);
	client.pay_share(&bill_one, &member_b);

	let settled_bill_one = client.get_bill(&bill_one);
	assert!(settled_bill_one.settled);
	assert_eq!(token_client.balance(&creator), 600);
}
