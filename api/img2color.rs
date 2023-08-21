use std::collections::HashMap;
use std::env;

use md5;
use reqwest::{self,Url};
use image::{self, DynamicImage, GenericImageView,imageops::FilterType};
use palette::LinSrgb;
use serde_json::json;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};
use redis::{Commands,Client};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

#[derive(Clone, Debug)]
struct Img {
    hex:String,
    color:String
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    // 连接数据库
    dotenv().ok();
    let redis_host = env::var("REDIS_HOST")?;
    let redis_port = env::var("REDIS_PORT")?;
    let username = env::var("USERNAME")?;
    let password = env::var("PASSWORD")?;
    
    let redis_url = format!("redis://{}:{}@{}:{}/",username,password,redis_host,redis_port);
    let db_client = Client::open(redis_url)?;
    let mut con = db_client.get_connection()?;
    
   // 解析请求
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

    // 查询缓存
    let img_hex: String = format!("{:?}",md5::compute(&img_url));
    let color:Option<String> = con.get(&img_hex)?;
    if let Some(i) = color {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(
                json!({
                    "RGB": &i
                })
                .to_string()
                .into(),
            )?
        );
    }

    // 下载解析图片
    let img:DynamicImage;
    match download_image_and_parse(img_url).await {
        Ok(i) => img = i,
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
    // 提取主题色 缓存并返回结果
    let img = Img {hex: img_hex,color: get_theme_color(&img).await};
    con.set(&img.hex,&img.color)?;
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
) -> Result<DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
    let url = fix_url(url).await;
    let resp = reqwest::get(&url).await?;
    let bytes = resp.bytes().await?;
    let img = image::load_from_memory(&bytes)?;
    let img= img.resize(50, (img.height()*50)/img.width(), FilterType::Lanczos3);
    Ok(img)
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
