#[macro_use]
extern crate dotenv_codegen;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use clap::Parser;
use google_sheets4::oauth2::{ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};
use google_sheets4::Sheets;
use hyper::Client;
use hyper_rustls::HttpsConnectorBuilder;
use serde::{Serialize, Serializer};
use serde::ser::{Error, SerializeMap};

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

#[derive(Clone, Debug)]
struct StringOrHashMap(Rc<dyn Any>);

impl Serialize for StringOrHashMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let string_type_id = TypeId::of::<String>();
        let hashmap_type_id = TypeId::of::<HashMap<String, StringOrHashMap>>();

        if (&*self.0).type_id() == string_type_id {
            let value = self.0.downcast_ref::<String>().unwrap();

            serializer.serialize_str(&value)
        }
        else if(&*self.0).type_id() == hashmap_type_id {
            let value = self.0.downcast_ref::<HashMap<String, StringOrHashMap>>().unwrap();

            let mut map = serializer.serialize_map(Some(value.len())).unwrap();
            for (k, v) in value {
                map.serialize_entry(&k.to_string(), &v)?;
            }
            map.end()
        }
        else {
            Err(Error::custom(""))
        }
    }
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

    let mut values = values.values.unwrap();

    let mut translations = HashMap::<String, HashMap<String, StringOrHashMap>>::new();

    let languages: Vec<String> = values[0][1..].to_vec();

    languages.iter().enumerate().for_each(|(i, language)| {
        let mut language_hashmap: HashMap<String, StringOrHashMap> = HashMap::new();

        values[1..].iter_mut().for_each(|strings| {
            while strings.len() < languages.len() + 1 {
                strings.push("".into());
            }

            let mut keys: Vec<String> = strings[0].split('.').map(|s| s.into()).collect();
            keys.reverse();

            if keys.len() == 1 {
                language_hashmap.insert(strings[0].clone(), StringOrHashMap(Rc::new(strings[i + 1].clone())));
                return;
            }

            let mut new_map: HashMap<String, StringOrHashMap> = HashMap::new();
            new_map.insert(keys[0].clone().to_string(), StringOrHashMap(Rc::new(strings[i + 1].clone())));

            keys[1..].iter().for_each(|key| {
                new_map.insert(key.clone(), StringOrHashMap(Rc::new(new_map.clone())));
                new_map.retain(|k, _| key == k);
            });

            language_hashmap = extended_string_or_hashmap(language_hashmap.clone(), new_map);
        });

        translations.insert(language.clone(), language_hashmap);
    });

    let json = serde_json::to_string_pretty(&translations).unwrap();

    fs::create_dir_all(&args.output.rsplit_once('/').unwrap_or(("", &args.output)).0).unwrap();
    let mut file = File::create(&args.output).unwrap();
    file.write_all(json.as_bytes()).unwrap();

    println!("Generated translation file ({}) for `{}`!", args.output, args.app);
}

fn extended_string_or_hashmap(hashmap: HashMap<String, StringOrHashMap>, new_hashmap: HashMap<String, StringOrHashMap>) -> HashMap<String, StringOrHashMap> {
    let mut extended = hashmap.clone();

    let repeating_keys = hashmap.keys().map(|k| {
        if new_hashmap.contains_key(k) {
            Some(k.clone())
        } else { None }
    })
        .filter(|k| k.is_some())
        .map(|k| k.unwrap())
        .collect::<Vec<String>>();

    new_hashmap.keys().filter(|key| !hashmap.contains_key(key.clone())).for_each(|key| {
        extended.insert(key.clone(), new_hashmap.get(key).unwrap().clone());
    });

    repeating_keys.iter().for_each(|key| {
        let first_node = hashmap.get(key).unwrap().0.downcast_ref::<HashMap<String, StringOrHashMap>>();
        let second_node = new_hashmap.get(key).unwrap().0.downcast_ref::<HashMap<String, StringOrHashMap>>();

        match (first_node, second_node) {
            (Some(f_node), Some(s_node)) => {
                extended.insert(key.clone(), StringOrHashMap(Rc::new(extended_string_or_hashmap(f_node.clone(), s_node.clone()))));
            }
            (Some(node), None) => {
                extended.insert(key.clone(), StringOrHashMap(Rc::new(node.clone())));
            }
            (None, Some(node)) => {
                extended.insert(key.clone(), StringOrHashMap(Rc::new(node.clone())));
            }
            (None, None) => {
                extended.insert(key.clone(), StringOrHashMap(Rc::new(hashmap.get(key).unwrap().0.downcast_ref::<String>().unwrap().clone())));
            }
        }
    });

    extended
}
