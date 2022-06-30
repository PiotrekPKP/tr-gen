#[macro_use]
extern crate dotenv_codegen;

use std::fs;
use clap::Parser;
use google_sheets4::oauth2::{ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};
use google_sheets4::Sheets;
use hyper::Client;
use hyper_rustls::HttpsConnectorBuilder;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the sheet to use (eg. `landing_page`)
    #[clap(short, long, value_parser)]
    app: String,

    /// Path to the output file
    #[clap(short, long, value_parser, default_value = "translations.json")]
    output: String,
}

#[tokio::main]
async fn main() {
    let args: Args = Args::parse();

    let client_id = dotenv!("CLIENT_ID");
    let client_secret = dotenv!("CLIENT_SECRET");
    let auth_uri = dotenv!("AUTH_URI");
    let token_uri = dotenv!("TOKEN_URI");

    let secret = ApplicationSecret {
        client_id: client_id.into(),
        client_secret: client_secret.into(),
        auth_uri: auth_uri.into(),
        token_uri: token_uri.into(),
        redirect_uris: vec![],
        ..Default::default()
    };

    let data_dir = dirs::data_local_dir().unwrap();
    let data_dir = data_dir.to_str().unwrap();

    fs::create_dir_all(format!("{}{}", data_dir, "/dinnery")).unwrap();
    let cache_path = format!("{}{}", data_dir, "/dinnery/tr-gen.cache");

    let auth = InstalledFlowAuthenticator::builder(
        secret,
        InstalledFlowReturnMethod::HTTPRedirect,
    )
        .persist_tokens_to_disk(cache_path)
        .build()
        .await
        .unwrap();

    let hub = Sheets::new(
        Client::builder().build(HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build()
        ),
        auth
    );

    let (_, values) = hub
        .spreadsheets()
        .values_get(dotenv!("SPREADSHEET_ID"), args.app.as_str())
        .doit()
        .await
        .unwrap();

    let values = values.values.unwrap();
}
