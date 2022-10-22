#![no_std]

use core::{panic};

use soroban_sdk::{
    contracterror, contractimpl, contracttype, map, symbol, vec, Address, BytesN, ConversionError,
    Env, RawVal, Symbol, TryFromVal, Vec,
};

mod token {
    soroban_sdk::contractimport!(file = "./soroban_token_spec.wasm");
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ProposalVote {
    pub voter: Address,
    pub prop_id: u32,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    TotSupply,
    Balance(Address),
    Bootstrap,
    Proposal(u32),
    ProposalId,
    Voted(ProposalVote),
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ProposalInstr {
    //contract id
    pub c_id: BytesN<32>,
    pub fun_name: Symbol,
    pub args: Vec<RawVal>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Proposal {
    pub tot_votes: i32,
    pub end_time: u64,
    // instrunctions will be executed in sequence
    pub instr: Vec<ProposalInstr>,
}

#[contracterror]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    IncorrectNonceForInvoker = 1,
    IncorrectNonce = 2,
}

pub trait DaoTrait {
    // fn test(env : Env) -> Address;
    fn init(env: Env);
    // transfer sahres if admin and if in bootstrap period
    fn x_shares(env: Env, amount: i32, to: Address);
    // fn vote(env : Env1)

    //create proposal and return its id
    fn c_prop(env: Env, proposal: Proposal) -> u32;

    //try to execute prop
    fn execute(env: Env, prop_id: u32);

    //get number of reputation or shares of the address
    fn shares(env: Env, of: Address) -> i32;

    //get total number of rep/shares that members hold
    fn tot_shares(env: Env) -> i32;

    //allow a member to vote on a proposal]
    fn vote(env: Env, prop_id: u32);
}

pub struct DaoContract;

#[contractimpl]
impl DaoTrait for DaoContract {
    fn init(env: Env) {
        if None == get_admin(&env) {
            // make the invoker the admin
            set_admin(&env, env.invoker());
            // give the invoker 1 "share" or "reputation"
            add_shares(&env, 1, env.invoker());

            // allow the admin to distributed shares for a week
            env.data()
                .set(DataKey::Bootstrap, env.ledger().timestamp() + 3600 * 24 * 7);
        }
    }

    fn x_shares(env: Env, amount: i32, to: Address) {
        // panic if not admin
        if !check_admin(&env) {
            panic!();
        }

        //panic if not in bootstrap period
        if !check_boot_strap(&env) {
            panic!()
        }
        add_shares(&env, amount, to)
    }

    fn c_prop(env: Env, proposal: Proposal) -> u32 {
        assert!(proposal.tot_votes == 0);

        let next_id = get_and_inc_prop_id(&env);

        env.data().set(DataKey::Proposal(next_id), proposal.clone());

        next_id
    }

    fn execute(env: Env, prop_id: u32) {
        let prop = env
            .data()
            .get::<_, Proposal>(DataKey::Proposal(prop_id))
            .unwrap()
            .unwrap();

        // can only execute before deadline
        assert!(prop.end_time > env.ledger().timestamp());
        // needs majority of rep/shares to execute
        assert!(prop.tot_votes > tot_shares(&env) / 2);

        //doesn't work
        // let allowed_contract_funs = map![&env, (symbol!("a_shares_c"), Self::a_shares_c)];

        for result in prop.instr {
            match result {
                Ok(instr) => {
                    if env.current_contract() == instr.c_id {
                        if instr.fun_name == symbol!("add_shares") {
                            let amount =
                                i32::try_from_val(&env, instr.args.get(0).unwrap().unwrap())
                                    .unwrap();
                            let to =
                                Address::try_from_val(&env, instr.args.get(1).unwrap().unwrap())
                                    .unwrap();
                            add_shares(&env, amount, to);
                        }
                    } else {
                        env.invoke_contract(&instr.c_id, &instr.fun_name, instr.args)
                    }
                }
                Err(_) => panic!(),
            }
        }
    }

    fn shares(env: Env, of: Address) -> i32 {
        get_shares(&env, of)
    }

    fn tot_shares(env: Env) -> i32 {
        tot_shares(&env)
    }

    fn vote(env: Env, prop_id: u32) {
        let mut prop = env
            .data()
            .get::<_, Proposal>(DataKey::Proposal(prop_id))
            .unwrap()
            .unwrap();

        // check if prop valid
        assert!(prop.end_time > env.ledger().timestamp());

        let member_shares = get_shares(&env, env.invoker());

        prop.tot_votes = prop.tot_votes + member_shares;

        env.data()
        .set(DataKey::Proposal(prop_id),prop);

        env.data().set(
            DataKey::Voted(ProposalVote {
                voter: env.invoker().clone(),
                prop_id,
            }),
            true,
        );
    }
}

fn get_and_inc_prop_id(env: &Env) -> u32 {
    let prev = env
        .data()
        .get(DataKey::ProposalId)
        .unwrap_or(Ok(0u32))
        .unwrap();

    env.data().set(DataKey::ProposalId, prev + 1);
    prev
}

fn check_boot_strap(env: &Env) -> bool {
    env.data()
        .get::<_, u64>(DataKey::Bootstrap)
        .unwrap()
        .unwrap()
        > env.ledger().timestamp()
}

fn get_shares(env: &Env, of: Address) -> i32 {
    env.data()
        .get(DataKey::Balance(of))
        .unwrap_or(Ok(0))
        .unwrap()
}

fn add_shares(env: &Env, amount: i32, to: Address) {
    let current_shares = env
        .data()
        .get(DataKey::Balance(to.clone()))
        .unwrap_or(Ok(0))
        .unwrap();

    env.data()
        .set(DataKey::Balance(to), amount + current_shares);

    update_tot_supply(env, amount)
}

fn update_tot_supply(env: &Env, amount: i32) {
    let total_shares = tot_shares(env);

    env.data().set(DataKey::TotSupply, total_shares + amount)
}

fn tot_shares(env: &Env) -> i32 {
    let total_shares = env.data().get(DataKey::TotSupply).unwrap_or(Ok(0)).unwrap();
    total_shares
}

fn check_admin(env: &Env) -> bool {
    env.invoker() == env.data().get(DataKey::Admin).unwrap().unwrap()
}

fn get_admin(env: &Env) -> Option<Result<Address, ConversionError>> {
    env.data().get(DataKey::Admin)
}

fn set_admin(env: &Env, admin: Address) {
    env.data().set(DataKey::Admin, admin)
}

#[cfg(test)]
mod test;
