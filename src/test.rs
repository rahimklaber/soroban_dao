#[cfg(test)]
use super::DaoContract;
use super::DaoContractClient;
use crate::{
    token::{self, Identifier, TokenMetadata, Signature},
    Address::Account,
    Proposal, ProposalInstr,
};
use soroban_sdk::{
    symbol,
    testutils::{Accounts, Ledger, LedgerInfo},
    vec, AccountId, Address, BytesN, Env, IntoVal, RawVal, BigInt,
};

fn create_token_contract(e: &Env, admin: &AccountId) -> (BytesN<32>, token::Client) {
    let id = e.register_contract_token(None);
    let token = token::Client::new(e, &id);
    // decimals, name, symbol don't matter in tests
    token.init(
        &Identifier::Account(admin.clone()),
        &TokenMetadata {
            name: "name".into_val(e),
            symbol: "symbol".into_val(e),
            decimals: 7,
        },
    );
    (id.into(), token)
}

fn create_dao_contract(e: &Env, admin: &AccountId) -> (BytesN<32>, DaoContractClient) {
    let contract_id = e.register_contract(None, DaoContract);
    let client = DaoContractClient::new(&e, &contract_id);
    client.with_source_account(admin).init();
    (contract_id.into(), client)
}

#[test]
fn test() {
    let env = Env::default();

    let user_1 = env.accounts().generate();

    let (dao_contract_id, client) = create_dao_contract(&env, &user_1);
    let (token_contract_id, token_client) = create_token_contract(&env, &user_1);

    client.with_source_account(&user_1).init();
    // check that init worked
    assert_eq!(
        1,
        client
            .with_source_account(&user_1)
            .shares(&Account(user_1.clone()))
    );
    //update time so that we are in the bootstrap period
    env.ledger().set(LedgerInfo {
        timestamp: env.ledger().timestamp() + 1,
        protocol_version: 1,
        sequence_number: 1,
        network_passphrase: Default::default(),
        base_reserve: 1,
    });

    // admin can give shares if in the bootstrap period
    client
        .with_source_account(&user_1)
        .x_shares(&100, &Account(user_1.clone()));

    //check that giving shares works in bootstrap period
    assert_eq!(
        101,
        client
            .with_source_account(&user_1)
            .shares(&Account(user_1.clone()))
    );

    // create proposal that executes code
    let prop = Proposal {
        tot_votes: 0,
        instr: vec![
            &env,
            ProposalInstr {
                c_id: dao_contract_id.clone().into(),
                fun_name: symbol!("add_shares"),
                args: vec![
                    &env,
                    (10i32).into_val(&env),
                    Address::Account(user_1.clone()).into_val(&env),
                ],
            },
        ],
        end_time: env.ledger().timestamp() + 1,
    };

    let prop_id = client.c_prop(&prop);

    client.with_source_account(&user_1).vote(&prop_id);

    client.execute(&prop_id);
    //check that executing proposal works
    assert_eq!(
        111,
        client
            .with_source_account(&user_1)
            .shares(&Account(user_1.clone()))
    );

    let prop = Proposal {
        tot_votes: 0,
        instr: vec![
            &env,
            ProposalInstr {
                c_id: token_contract_id.clone().into(),
                fun_name: symbol!("xfer"),
                args: vec![
                    &env,
                    (Signature::Invoker).into_val(&env),
                    (BigInt::from_i32(&env, 0)).into_val(&env),
                    Address::Account(user_1.clone()).into_val(&env), //dest
                    BigInt::from_u64(&env, 9).into_val(&env)
                ],
            },
        ],
        end_time: env.ledger().timestamp() + 1,
    };
    // give dao contract tokens
    token_client.with_source_account(&user_1).mint(&Signature::Invoker, &BigInt::from_i32(&env, 0), &Identifier::Contract(dao_contract_id.clone()),  &BigInt::from_u64(&env, 10));
    assert_eq!(BigInt::from_u64(&env, 10), token_client.balance(&Identifier::Contract(dao_contract_id.clone())) );
    let prop_id = client.c_prop(&prop);

    client.with_source_account(&user_1).vote(&prop_id);
    // execute proposal that transfers tokens from dao.
    client.execute(&prop_id);

    assert_eq!(BigInt::from_u64(&env, 9), token_client.balance(&Identifier::Account(user_1.clone())) );

    assert_eq!(BigInt::from_u64(&env, 1), token_client.balance(&Identifier::Contract(dao_contract_id.clone())) );


}
