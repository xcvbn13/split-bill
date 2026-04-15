# Split Bill Smart Contract (Soroban)

## Application Description

This project is a Soroban smart contract that supports split-bill payments on Stellar.

One user (the creator) pays first, then each member must reimburse their share before a deadline. The contract keeps funds in escrow, applies a late penalty automatically when needed, and transfers the total amount to the creator once all members have paid.

## Features

- Multi-bill support in a single contract instance.
- Per-member split amounts for each bill.
- Payment deadline per bill.
- Automatic late-penalty calculation.
- Escrow model using Soroban token transfers.
- Automatic settlement to the creator when all members are paid.
- Bill status tracking (paid, unpaid, late, total collected, settled).

## Smart Contract (Testnet)

- Contract ID (Testnet): `CDRPHVZXXQMTQBL56YDNUL44PFYVPJT4ASNZNAKV22KG5T22PZD757F6`

## Public Contract Methods

- `create_bill(creator, token, members, amounts, deadline, penalty_percent) -> bill_id`
- `pay_share(bill_id, member)`
- `get_bill(bill_id) -> Bill`
- `get_member_due(bill_id, member) -> i128`

## Testnet Screenshot

Below is a placeholder screenshot section for your deployed testnet contract view:

![Testnet Contract Screenshot](screenshot.png)

## Running Tests

```bash
cd contracts/notes
cargo test
```

Current test coverage includes:

- On-time payment settlement.
- Late payment with penalty.
- Multiple active bills isolation.
