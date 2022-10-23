### Soroban Reputation DAO

## What is this?

This is an example DAO contract. It allows members to create proposals and execute them if enough members have voted. The DAO uses non-transferable reputation to model membership.

The idea I had in my mind while creating this was that every week or month a new proposals is created to reward members with reputation depending on how much work they did for the DAO. The proposal could also remove reputation if not enough was done by some members.

## Explanation

### Creating the contract

To initialize the contract, the `init` function should be called.



```rust
fn init(env: Env) {
        if None != get_admin(&env) {
            panic!();
        }
        // make the invoker the admin
        set_admin(&env, env.invoker());
        // give the invoker 1 "share" or "reputation"
        add_shares(&env, 1, env.invoker());

        // allow the admin to distributed shares for a week
        env.data()
            .set(DataKey::Bootstrap, env.ledger().timestamp() + 3600 * 24 * 7);
    }
```
The `init` function does the following:
1. The code checks whether the contract has allready been intialized, and if so panics.
2. The invoker of the `init` function is made the admin.
3. The admin is given one reputation/share in to become member of the DAO.
4. The bootstrap period is set to 1 week. This allows the admin make users DAO members by using the `x_shares` function.

### Bootstrapping
To facilitate the decentralization of the DAO, the admin is able to use the `x_shares` function for 1 week to give users reputation and therefore make them DAO members.


```rust
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
```

The `x_shares` function does the following:
1. The function checks whether the invoker is the admin, and if not panics.
2. The function checks whether the bootstrap period has passed, and if so panics.
3. The function gives `amount` of reputation to `to`.
### Creating a proposal

Anyone can create proposals. In a real world scenario this might be restricted to members with a certain amount of reputation.

The code block below shows the contract function which is used to register a proposal with the contract.


``` rust
fn c_prop(env: Env, proposal: Proposal) -> u32 {
        assert!(proposal.tot_votes == 0);

        let next_id = get_and_inc_prop_id(&env);

        env.data().set(DataKey::Proposal(next_id), proposal.clone());

        next_id
    }
```
The `c_prop` function does the following:
1. The function checks whether the proposal has 0 votes, and panics if not. The function needs to check this since the votes are stored in the Proposal struct, which was created by the invoker of the function and thus could have set the total votes to be higher.
2. The function generates an id for the proposal.
3. The function stores the proposal.
4. The function returns the id of the proposal.

Before adding a proposal to the contract, it should be created. 

A proposal is represented as a struct with with 3 members:
1. `tot_votes:` represents the amount of votes.
2. `instr`: a list of `ProposalInstr`, which holds information about which functions of which contracts to call and what arguments to provide.
    - `c_id`: the id of the contract whose function we want to call.
    - `fun_name` the name of the function we want to call.
    - `args` : the arguments of the function we want to call.
3. `end_time`: unix time untill the proposal is still valid.

In code block below, a proposal struct is created that when executed will send 9 tokens that the DAO contract owns to `user_1`. This is done by making the proposal invoke the `xfer` function of the token contract.

```rust 
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
```
### Voting on proposal

Members of the DAO can vote by calling the `vote` function and indicating which proposal to vote for.

```rust
 fn vote(env: Env, prop_id: u32) {
        assert!(!voted(&env, env.invoker(), prop_id));

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
                voter: env.invoker(),
                prop_id,
            }),
            true,
        );
    }
```


 The `function` does the following:
 - the function checks that the proposal has not allready been voted on by this member.
 - the function retrieves the proposal.
 - the function checks that the proposal has not expired.
 - the function retrieves the reputation of the voter.
 - the function adds the repuation of the voter to the total votes of the proposal.
 - the function stores the updated proposal.
 - the function stores that the voter has voted for the proposal.

 ### Executing a proposal
 After a proposal has enough votes, it can be executed. Proposals need a majority of votes to be executed.

 To do this, anyone can call the `execute` function while specifying the proposal id.

 ```rust
fn execute(env: Env, prop_id: u32) {

        // can only execute once
        assert!(!get_executed(&env, prop_id));

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
        set_executed(&env, prop_id);
    }
```

The `execute` function does the following:
- the contract checks whether the proposal was executed allready, and if so panics. 
- the function retrieves the proposal.
- the function checks whether the proposal is still valid.
- the function checks whether a majority of the members have voted for the proposal.
- the function loops over and executes the instructions contained in the proposal. It is not possible to use `env.invoke_contract` to invoke a function from the current contract, so if we want to use to proposal to give a member reputation, this is handled seperately.
- the function stores that the proposal was executed.
