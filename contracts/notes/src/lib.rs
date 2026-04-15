#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Bill(u64),
    NextBillId,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberState {
    pub member: Address,
    pub base_amount: i128,
    pub paid_amount: i128,
    pub paid: bool,
    pub late: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bill {
    pub creator: Address,
    pub token: Address,
    pub members: Vec<MemberState>,
    pub deadline: u64,
    pub penalty_percent: u32,
    pub total_collected: i128,
    pub settled: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SplitBillError {
    BillNotFound = 1,
    InvalidInput = 2,
    MemberNotFound = 3,
    MemberAlreadyPaid = 4,
    BillAlreadySettled = 5,
    Overflow = 6,
}

#[contract]
pub struct SplitBillContract;

#[contractimpl]
impl SplitBillContract {
    pub fn create_bill(
        env: Env,
        creator: Address,
        token: Address,
        members: Vec<Address>,
        amounts: Vec<i128>,
        deadline: u64,
        penalty_percent: u32,
    ) -> u64 {
        if members.len() == 0 || members.len() != amounts.len() || penalty_percent > 100 {
            panic_with_error!(&env, SplitBillError::InvalidInput);
        }

        creator.require_auth();

        let mut member_states = Vec::<MemberState>::new(&env);
        for i in 0..members.len() {
            let member = members.get(i).unwrap();
            let amount = amounts.get(i).unwrap();
            if amount <= 0 {
                panic_with_error!(&env, SplitBillError::InvalidInput);
            }

            member_states.push_back(MemberState {
                member,
                base_amount: amount,
                paid_amount: 0,
                paid: false,
                late: false,
            });
        }

        let bill = Bill {
            creator,
            token,
            members: member_states,
            deadline,
            penalty_percent,
            total_collected: 0,
            settled: false,
        };

        let bill_id = Self::next_bill_id(&env);
        env.storage().instance().set(&DataKey::Bill(bill_id), &bill);
        bill_id
    }

    pub fn pay_share(env: Env, bill_id: u64, member: Address) {
        let mut bill = Self::get_bill_or_panic(&env, bill_id);
        if bill.settled {
            panic_with_error!(&env, SplitBillError::BillAlreadySettled);
        }

        member.require_auth();

        let mut member_state = None;
        let mut member_index = 0u32;

        for i in 0..bill.members.len() {
            let item = bill.members.get(i).unwrap();
            if item.member == member {
                member_state = Some(item);
                member_index = i;
                break;
            }
        }

        let mut state = match member_state {
            Some(v) => v,
            None => panic_with_error!(&env, SplitBillError::MemberNotFound),
        };

        if state.paid {
            panic_with_error!(&env, SplitBillError::MemberAlreadyPaid);
        }

        let due = Self::calculate_due(&env, state.base_amount, bill.deadline, bill.penalty_percent);

        token::Client::new(&env, &bill.token).transfer(
            &member,
            &env.current_contract_address(),
            &due,
        );

        state.paid = true;
        state.late = env.ledger().timestamp() > bill.deadline;
        state.paid_amount = due;
        bill.members.set(member_index, state);

        bill.total_collected = match bill.total_collected.checked_add(due) {
            Some(v) => v,
            None => panic_with_error!(&env, SplitBillError::Overflow),
        };

        if Self::all_members_paid(&bill.members) {
            token::Client::new(&env, &bill.token).transfer(
                &env.current_contract_address(),
                &bill.creator,
                &bill.total_collected,
            );
            bill.settled = true;
        }

        env.storage().instance().set(&DataKey::Bill(bill_id), &bill);
    }

    pub fn get_bill(env: Env, bill_id: u64) -> Bill {
        Self::get_bill_or_panic(&env, bill_id)
    }

    pub fn get_member_due(env: Env, bill_id: u64, member: Address) -> i128 {
        let bill = Self::get_bill_or_panic(&env, bill_id);
        for i in 0..bill.members.len() {
            let item = bill.members.get(i).unwrap();
            if item.member == member {
                if item.paid {
                    return 0;
                }
                return Self::calculate_due(&env, item.base_amount, bill.deadline, bill.penalty_percent);
            }
        }
        panic_with_error!(&env, SplitBillError::MemberNotFound)
    }

    fn get_bill_or_panic(env: &Env, bill_id: u64) -> Bill {
        match env.storage().instance().get::<DataKey, Bill>(&DataKey::Bill(bill_id)) {
            Some(v) => v,
            None => panic_with_error!(env, SplitBillError::BillNotFound),
        }
    }

    fn next_bill_id(env: &Env) -> u64 {
        let current = env
            .storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::NextBillId)
            .unwrap_or(1);

        let next = match current.checked_add(1) {
            Some(v) => v,
            None => panic_with_error!(env, SplitBillError::Overflow),
        };

        env.storage().instance().set(&DataKey::NextBillId, &next);
        current
    }

    fn all_members_paid(members: &Vec<MemberState>) -> bool {
        for i in 0..members.len() {
            if !members.get(i).unwrap().paid {
                return false;
            }
        }
        true
    }

    fn calculate_due(env: &Env, base_amount: i128, deadline: u64, penalty_percent: u32) -> i128 {
        if env.ledger().timestamp() <= deadline {
            return base_amount;
        }

        let penalty = match base_amount.checked_mul(penalty_percent as i128) {
            Some(v) => v / 100,
            None => panic_with_error!(env, SplitBillError::Overflow),
        };

        match base_amount.checked_add(penalty) {
            Some(v) => v,
            None => panic_with_error!(env, SplitBillError::Overflow),
        }
    }
}

mod test;
