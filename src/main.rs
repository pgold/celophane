use anyhow::Result;
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
    address: Address,
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

async fn print_balance<M: Middleware>(
    maybe_token: Result<celo::Erc20<M>>,
    address: Address,
    label: &str,
) -> Result<()> {
    // One would expect a non-existent token to be an Error on maybe_token,
    // but there is a token and the balance_of call fails instead.
    match maybe_token {
        Ok(token) => match token.balance_of(address).call().await {
            Ok(balance) => println!("{}: {}", label, balance),
            // Ignore unexistent token.
            Err(_) => (),
        },
        // Ignore unexistent token.
        Err(_) => (),
    }
    Ok(())
}

async fn account_balance<M: Middleware>(client: Arc<M>, args: AccountBalanceOpt) -> Result<()> {
    let celo = celo::get_celo_token(client.clone()).await;
    let cusd = celo::get_cusd_token(client.clone()).await;
    let ceur = celo::get_ceur_token(client.clone()).await;

    println!("All balances expressed in units of 10^-18.");
    print_balance(celo, args.address, "CELO").await?;
    print_balance(cusd, args.address, "cUSD").await?;
    print_balance(ceur, args.address, "cEUR").await?;

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
