use anyhow::{anyhow, Context, Result};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{Address, U256};
use std::convert::TryFrom;
use std::sync::Arc;
use structopt::StructOpt;

mod celo;

#[derive(StructOpt)]
struct CelophaneOpt {
    /// Endpoint to connect to.
    #[structopt(long, default_value = "http://localhost:8545")]
    endpoint: String,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
struct AccountBalanceOpt {
    address: String,
}

#[derive(StructOpt)]
enum AccountCommand {
    /// Retrieve an account's balance.
    Balance(AccountBalanceOpt),
}

#[derive(StructOpt)]
struct ExchangeShowOpt {
    /// Amount (base) to report quote amounts on.
    #[structopt(long, default_value = "1000000000000000000", parse(try_from_str=U256::from_dec_str))]
    amount: U256,
}

#[derive(StructOpt)]
enum ExchangeCommand {
    /// Display the on-chain exchange (Mento) rates.
    Show(ExchangeShowOpt),
}

#[derive(StructOpt)]
enum Command {
    /// Account management.
    Account(AccountCommand),
    /// On-chain exchange (Mento) interaction.
    Exchange(ExchangeCommand),
}

async fn account_balance<M: Middleware>(client: Arc<M>, args: AccountBalanceOpt) -> Result<()> {
    let address = args
        .address
        .parse::<Address>()
        .with_context(|| "failed to parse account address")?;

    let celo = celo::get_celo_token(client.clone()).await.unwrap();
    let cusd = celo::get_cusd_token(client.clone()).await.unwrap();
    let celo_balance = celo.balance_of(address).call().await.unwrap();
    let cusd_balance = cusd.balance_of(address).call().await.unwrap();

    println!("All balances expressed in units of 10^-18.");
    println!("CELO: {}", celo_balance);
    println!("cUSD: {}", cusd_balance);
    Ok(())
}

async fn exchange_show<M: Middleware>(client: Arc<M>, args: ExchangeShowOpt) -> Result<()> {
    let exchange = celo::get_exchange(client.clone()).await.unwrap();

    let base_qty = args.amount;

    let cusd_quote_qty = exchange
        .get_buy_token_amount(base_qty, true)
        .call()
        .await
        .unwrap();
    let celo_quote_qty = exchange
        .get_buy_token_amount(base_qty, false)
        .call()
        .await
        .unwrap();

    println!("{} CELO => {} cUSD", base_qty, cusd_quote_qty);
    println!("{} cUSD => {} CELO", base_qty, celo_quote_qty);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CelophaneOpt::from_args();

    let provider = Provider::<Http>::try_from(args.endpoint)?;
    let client = Arc::new(provider);

    match args.cmd {
        Command::Account(opt) => match opt {
            AccountCommand::Balance(opt) => account_balance(client, opt).await?,
        },
        Command::Exchange(opt) => match opt {
            ExchangeCommand::Show(opt) => exchange_show(client, opt).await?,
        },
    }

    Ok(())
}
