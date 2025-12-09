use sendspin::protocol::client::ProtocolClient;
use sendspin::protocol::messages::{AudioFormatSpec, ClientHello, DeviceInfo, PlayerSupport};

const DEFAULT_SERVER: &str = "ws://localhost:8927/sendspin";
const DEFAULT_NAME: &str = "Sendspin-RS Client";

fn parse_args() -> (String, String) {
    let mut server = DEFAULT_SERVER.to_string();
    let mut name = DEFAULT_NAME.to_string();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--server" | "-s" => {
                if let Some(value) = args.next() {
                    server = value;
                }
            }
            "--name" | "-n" => {
                if let Some(value) = args.next() {
                    name = value;
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {}
        }
    }

    (server, name)
}

fn print_usage() {
    println!(
        "Usage: sendspin [--server <url>] [--name <client name>]\n\
        \n\
        Connect to a Sendspin server and perform the initial handshake.\n\
        Defaults: server={DEFAULT_SERVER}, name=\"{DEFAULT_NAME}\"."
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (server, name) = parse_args();

    println!("Connecting to {server} as {name}...");

    let hello = ClientHello {
        client_id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        version: 1,
        supported_roles: vec!["player@v1".to_string()],
        device_info: DeviceInfo {
            product_name: name.clone(),
            manufacturer: "Sendspin".to_string(),
            software_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        player_support: Some(PlayerSupport {
            supported_formats: vec![
                AudioFormatSpec {
                    codec: "pcm".to_string(),
                    channels: 2,
                    sample_rate: 48_000,
                    bit_depth: 24,
                },
                AudioFormatSpec {
                    codec: "pcm".to_string(),
                    channels: 2,
                    sample_rate: 96_000,
                    bit_depth: 24,
                },
            ],
            // Buffer capacity in bytes (per spec) - 200KB buffer
            buffer_capacity: 200_000,
            supported_commands: vec!["volume".to_string(), "mute".to_string()],
        }),
        metadata_support: None,
    };

    let _client = ProtocolClient::connect(&server, hello).await?;

    println!("Connected! Waiting for server hello...");

    Ok(())
}
