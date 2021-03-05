use anyhow::Result;
use ethers::prelude::abigen;
use ethers::providers::Middleware;
use ethers::types::Address;
use std::sync::Arc;

const REGISTRY_ADDRESS: &str = "000000000000000000000000000000000000ce10";

const EXCHANGE: &str = "Exchange";
const GOLD_TOKEN: &str = "GoldToken";
const STABLE_TOKEN: &str = "StableToken";
const STABLE_TOKEN_EUR: &str = "StableTokenEUR";

abigen!(Erc20, "./src/abis/IERC20.json");
abigen!(Registry, "./src/abis/Registry.json");
abigen!(Exchange, "./src/abis/Exchange.json");

async fn get_erc20_token<M: Middleware>(client: Arc<M>, name: &str) -> Result<Erc20<M>> {
    let address = registry_lookup(client.clone(), name).await?;
    let token = Erc20::new(address, client.clone());
    Ok(token)
}

pub async fn get_celo_token<M: Middleware>(client: Arc<M>) -> Result<Erc20<M>> {
    get_erc20_token(client, GOLD_TOKEN).await
}

pub async fn get_cusd_token<M: Middleware>(client: Arc<M>) -> Result<Erc20<M>> {
    get_erc20_token(client, STABLE_TOKEN).await
}

pub async fn get_ceur_token<M: Middleware>(client: Arc<M>) -> Result<Erc20<M>> {
    get_erc20_token(client, STABLE_TOKEN_EUR).await
}

pub async fn get_exchange<M: Middleware>(client: Arc<M>) -> Result<Exchange<M>> {
    let exchange_address = registry_lookup(client.clone(), EXCHANGE).await.unwrap();
    let exchange = Exchange::new(exchange_address, client.clone());
    Ok(exchange)
}

pub async fn registry_lookup<M: Middleware>(client: Arc<M>, name: &str) -> Result<Address> {
    let registry_address: Address = REGISTRY_ADDRESS.parse()?;
    let registry = Registry::new(registry_address, client);
    let address = registry
        .get_address_for_string(name.to_string())
        .call()
        .await
        .unwrap();
    Ok(address)
}
