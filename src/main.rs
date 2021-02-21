use anyhow::{anyhow, Context, Result};
use ethers::prelude::*;
use ethers::providers::{Http, JsonRpcClient, Provider};
use ethers::types::Address;
use std::convert::TryFrom;
use std::sync::Arc;
use structopt::StructOpt;

const REGISTRY_ADDRESS: &str = "000000000000000000000000000000000000ce10";

const EXCHANGE: &str = "Exchange";
const GOLD_TOKEN: &str = "GoldToken";
const STABLE_TOKEN: &str = "StableToken";

abigen!(Erc20, "./src/abis/IERC20.json");

abigen!(Registry, "./src/abis/Registry.json");

abigen!(Exchange, "./src/abis/Exchange.json");

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

async fn get_erc20_token<T: JsonRpcClient>(
    client: Arc<Provider<T>>,
    name: &str,
) -> Result<Erc20<Provider<T>>> {
    let address = registry_lookup(client.clone(), name).await?;
    let token = Erc20::new(address, client.clone());
    Ok(token)
}

async fn registry_lookup<T: JsonRpcClient>(
    client: Arc<Provider<T>>,
    name: &str,
) -> Result<Address> {
    let registry_address: Address = REGISTRY_ADDRESS.parse()?;
    let registry = Registry::new(registry_address, client);
    let address = registry
        .get_address_for_string(name.to_string())
        .call()
        .await
        .unwrap();

    Ok(address)
}

async fn account_balance<T: JsonRpcClient>(
    client: Arc<Provider<T>>,
    args: AccountBalanceOpt,
) -> Result<()> {
    let address = args
        .address
        .parse::<Address>()
        .with_context(|| "failed to parse account address")?;

    let celo = get_erc20_token(client.clone(), GOLD_TOKEN).await.unwrap();
    let cusd = get_erc20_token(client.clone(), STABLE_TOKEN).await.unwrap();
    let celo_balance = celo.balance_of(address).call().await.unwrap();
    let cusd_balance = cusd.balance_of(address).call().await.unwrap();

    println!("All balances expressed in units of 10^-18.");
    println!("CELO: {}", celo_balance);
    println!("cUSD: {}", cusd_balance);
    Ok(())
}

async fn exchange_show<T: JsonRpcClient>(
    client: Arc<Provider<T>>,
    args: ExchangeShowOpt,
) -> Result<()> {
    let mento_address = registry_lookup(client.clone(), EXCHANGE).await.unwrap();
    let exchange = Exchange::new(mento_address, client.clone());

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
