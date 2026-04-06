# redeem-rs

A service that fetches redeemable [Circles](https://aboutcircles.com) subscriptions from the SubIndexer API and redeems them on-chain via the `SubscriptionModule` contract on Gnosis Chain.

## Configuration

| Variable  | Required | Default                            | Description                         |
|-----------|----------|------------------------------------|-------------------------------------|
| `PK`      | Yes      | —                                  | Private key of the redeeming wallet |
| `API_URL` | No       | `http://localhost:3030/redeemable` | SubIndexer redeemable endpoint      |

Copy `.env.sample` to `.env` and fill in your values, or export the variables directly.

## Usage

```bash
cargo run
```

## Testing

```bash
# Unit tests (no network required)
cargo test

# Integration test — redeems the first subscription from the API
cargo test test_redeem_one -- --ignored
```
