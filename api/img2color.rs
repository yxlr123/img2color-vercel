use std::collections::HashMap;
use std::env;

use md5;
use reqwest::{self,Url};
use image::{self, DynamicImage, GenericImageView,imageops::FilterType};
use palette::LinSrgb;
use serde_json::json;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};
use mongodb::{Client, options::{ClientOptions,ServerApi, ServerApiVersion},bson::doc};
use serde::{Deserialize,Serialize};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Img {
    hex:String,
    color:String
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    dotenv().ok();
    let username = env::var("USERNAME").unwrap(); 
    let password = env::var("PASSWORD").unwrap();
    let host = env::var("MONGO_HOST").unwrap();

    let url = format!(
        "mongodb+srv://{}:{}@{}/?retryWrites=true&w=majority", 
        username, password, host
    );

    // 创建客户端
    let mut client_options = ClientOptions::parse(&url).await?;
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    let db_client = Client::with_options(client_options)?;
    let db = db_client.database("admin");
    let collection = db.collection::<Img>("colors");

    let parsed_url = Url::parse(&req.uri().to_string()).unwrap();
    let hash_query: HashMap<String, String> = parsed_url.query_pairs().into_owned().collect();
    let img = hash_query.get("img");
    let img_url: &str;
    match img {
        Some(u) => {
            img_url = u;
        }
        None => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(
                    json!({
                        "error":"请正确输入参数"
                    })
                    .to_string()
                    .into(),
                )?);
        }
    };
    let img: DynamicImage;
    let img_hex: String;
    match download_image_and_parse(img_url).await {
        Ok(i) => (img,img_hex) = i,
        Err(e) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(
                    json!({
                        "error":e.to_string()
                    })
                    .to_string()
                    .into(),
                )?)
        }
    }
    let cursor = collection.find(doc! {"hex":&img_hex}, None).await?;
    if let Ok(i) = cursor.deserialize_current() {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(
                json!({
                    "RGB": &i.color
                })
                .to_string()
                .into(),
            )?
        );
    }
    let img = Img {hex: img_hex,color: get_theme_color(&img).await};
    collection.insert_one(&img,None).await?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(
            json!({
              "RGB": &img.color
            })
            .to_string()
            .into(),
        )?)
}

async fn fix_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("http://{}", url)
    }
}

async fn download_image_and_parse(
    url: &str,
) -> Result<(DynamicImage,String), Box<dyn std::error::Error + Send + Sync>> {
    let url = fix_url(url).await;
    let resp = reqwest::get(url).await?;
    let bytes = resp.bytes().await?;
    let img_hex = md5::compute(&bytes);
    let img = image::load_from_memory(&bytes)?;
    let img= img.resize(50, (img.height()*50)/img.width(), FilterType::Lanczos3);
    Ok((img,format!("{:?}",img_hex)))
}

async fn get_theme_color(img: &DynamicImage) -> String {
    // Get the image dimensions
    let (width, height) = img.dimensions();
    // Calculate the average color of the image
    let mut sum_red: u32 = 0;
    let mut sum_green: u32 = 0;
    let mut sum_blue: u32 = 0;

    for x in 0..width {
        for y in 0..height {
            let pixel = img.get_pixel(x, y);
            sum_red += pixel[0] as u32;
            sum_green += pixel[1] as u32;
            sum_blue += pixel[2] as u32;
        }
    }

    let pixel_count = (width * height) as f32;
    let avg_red = (sum_red as f32 / pixel_count).round() as u8;
    let avg_green = (sum_green as f32 / pixel_count).round() as u8;
    let avg_blue = (sum_blue as f32 / pixel_count).round() as u8;

    // Create a palette color from the average color
    let avg_color = LinSrgb::new(
        avg_red as f32 / 255.0,
        avg_green as f32 / 255.0,
        avg_blue as f32 / 255.0,
    );

    // Convert the color to hexadecimal format
    format!("#{:X}", avg_color.into_format::<u8>())
}
