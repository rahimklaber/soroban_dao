#[cfg(test)]
use super::{DaoContract};
use super::DaoContractClient;
use soroban_sdk::{symbol, vec, Env, testutils::{Accounts, Ledger, LedgerInfo}, RawVal, Address, IntoVal};
use crate::{Address::Account, Proposal, ProposalInstr};

   
#[test]
fn test() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DaoContract);
    

    
    let user_1 = env.accounts().generate();

    let client = DaoContractClient::new(&env, &contract_id);

    client.with_source_account(&user_1).init();

    assert_eq!(1,client.with_source_account(&user_1).shares(&Account(user_1.clone())));

    env.ledger().set(LedgerInfo{
        timestamp : env.ledger().timestamp() + 1,
        protocol_version: 1,
        sequence_number: 1,
        network_passphrase: Default::default(),
        base_reserve: 1,
    });

    client.with_source_account(&user_1).x_shares(&100,&Account(user_1.clone()));

    assert_eq!(101,client.with_source_account(&user_1).shares(&Account(user_1.clone())));

    let prop = Proposal{
        tot_votes: 0,
        instr: vec![&env,ProposalInstr{ c_id: contract_id.clone(), fun_name: symbol!("add_shares"), args:vec![&env,(10i32).into_val(&env),Address::Account(user_1.clone()).into_val(&env)]}],
    };

    let prop_id = client.c_prop(&prop);

    client.execute(&prop_id);

    assert_eq!(111,client.with_source_account(&user_1).shares(&Account(user_1.clone())));

}