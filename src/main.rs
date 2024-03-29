use walkdir::WalkDir;
use regex::Regex;
use home::home_dir;
use std::env;
use tokio::fs::File;
use std::fs::{OpenOptions,read_to_string,write};
use reqwest::{Body, Client, multipart};
use tokio_util::codec::{BytesCodec, FramedRead};
use dotenv::dotenv;

pub async fn get_cache() -> Vec<String> {
    let _ = OpenOptions::new().write(true).create_new(true).open(format!("{}/.config/steam-sync.txt", home_dir().unwrap().display()));

    let cache_file = read_to_string(format!("{}/.config/steam-sync.txt", home_dir().unwrap().display())).unwrap();
    let cache: Vec<&str> = cache_file.split(';').collect();
    return cache.iter().map(|&x| x.to_owned()).collect();
}

pub async fn add_to_cache(filename: &String) -> Result<(), &'static str> {
    let mut cache = get_cache().await;
    cache.push(filename.to_owned());
    match write(format!("{}/.config/steam-sync.txt", home_dir().unwrap().display()), cache.join(";")) {
        Ok(_) => Ok(()),
        Err(_) => Err("Error writting file")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let key = std::env::var("KEY").expect("didn't provide key");

    let re = Regex::new(r#"screenshots/[^t]"#).unwrap();

    let cache = get_cache().await;
    for e in WalkDir::new(format!("{}/.local/share/Steam/userdata/", home_dir().unwrap().display())).into_iter().filter_map(|e| e.ok()) {
        if re.is_match(e.path().to_str().unwrap()) && e.path().is_file() {
            let filename: String = e.path().file_name().unwrap().to_str().unwrap().to_string();
            match cache.contains(&filename) {
                false => {
                    let mut url = env::var("SERVER").unwrap();
                    url.push('/');
                    url.push_str(&filename);
                    let client = Client::builder()
                        .danger_accept_invalid_certs(true)
                        .build()?;

                    let response = client.get(&url).header("Auth-token", &key).send().await?;

                    match response.status() {
                        reqwest::StatusCode::NOT_FOUND => {
                            let file = File::open(e.path()).await?;

                            // read file body stream
                            let stream = FramedRead::new(file, BytesCodec::new());
                            let file_body = Body::wrap_stream(stream);

                            //make form part of file
                            let some_file = multipart::Part::stream(file_body)
                                .file_name(filename.to_owned());

                            //create the multipart form
                            let form = multipart::Form::new()
                                .part("file", some_file);

                            let url = env::var("SERVER").unwrap().to_string();
                            //send request
                            let response = client.put(url.to_owned()).multipart(form).header("Auth-token", &key).send().await?;
                            println!("Uploading file {}", &filename);
                            match response.status() {
                                reqwest::StatusCode::OK => add_to_cache(&filename).await?,
                                res => println!("Error uploading file {}. {} {}", &filename, &url, res)
                            }
                        },
                        reqwest::StatusCode::OK => add_to_cache(&filename).await?,
                        _ => {}
                    }
                },
                true => {}
            }
        }
    }
    Ok(())
}
