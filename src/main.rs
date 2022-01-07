use dotenv::dotenv;
use std::env;
use anyhow::Error as AnyError;
use pnet::{ datalink, ipnetwork };
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
enum ClientError {
    #[error("Please select the right device.")]
    MissingDevice,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

fn get_ip_by_device_name(dev_name: String) -> Option<ipnetwork::IpNetwork> {
    datalink::interfaces()
        .into_iter()
        .find(|iface| iface.name == dev_name)

    .map(|iface| iface.ips.into_iter().find(|ip| ip.is_ipv4()))
    .flatten()
}

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    dotenv().ok();

    println!("Hello, world!");

    let zone = env::var("ZONE").unwrap();
    let token = env::var("TOKEN").unwrap();
    let target_name = env::var("TARGET_NAME").unwrap();
    let device_name = env::var("DEVICE").unwrap();

    let target_ip = get_ip_by_device_name(device_name).ok_or(ClientError::MissingDevice)?.ip();
    println!("{}", target_ip);

    update_dns(zone, token, target_name, target_ip.to_string()).await?;

    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
struct DNSRecords {
    content: String,
    id: String,
    #[serde(rename = "type")]
    _type: String,
    name: String,
    ttl: u32,
}

#[derive(Deserialize, Debug)]
struct RetValue {
    result: Vec<DNSRecords>,
}

async fn update_dns(zone: String, token: String, target_name: String, target_ip: String ) -> Result<(), AnyError> {

    let client = reqwest::Client::new();
    let body = client
        .get(format!(
            "{}/{}/{}",
            "https://api.cloudflare.com/client/v4/zones", zone, "dns_records"
        ))
        .query(&[("name",&target_name)])
        .bearer_auth(&token)
        .send()
        .await?
        .json::<RetValue>()
        .await?;

    let mut record = body
        .result
        .into_iter()
        .find(|record| record.name == target_name)
        .unwrap();

    if record.content == target_ip {
        println!("no need to update");
        return Ok(());
    }

    record.content = target_ip;
    println!("body = {:#?}", record);

    let ret = client
        .patch(format!(
            "{}/{}/{}/{}",
            "https://api.cloudflare.com/client/v4/zones",
            zone, "dns_records", record.id
        ))
        .json(&record)
        .bearer_auth(token)
        .send()
        .await?
        .json::<Value>()
        .await?;


    println!("{:?}", ret);
    Ok(())
}
