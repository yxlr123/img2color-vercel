use std::collections::HashMap;
use std::io;

use hyper::{Body as hBody, Client, Request as hRequest, Uri};
use hyper_tls::HttpsConnector;
use image::{io::Reader as ImageReader, DynamicImage, GenericImageView};
use palette::LinSrgb;
use serde_json::json;
use url::Url;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
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
    match download_image_and_parse(img_url).await {
        Ok(i) => img = i,
        Err(_e) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(
                    json!({
                        "error":"图片不存在"
                    })
                    .to_string()
                    .into(),
                )?)
        }
    }
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(
            json!({
              "RGB": get_theme_color(img).await
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
    // 将URL解析为Uri类型
    let url = fix_url(url).await;
    let uri = url.parse::<Uri>()?;

    // 创建一个新的hyper客户端
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    // 创建一个新的请求
    let request = hRequest::builder()
        .uri(uri)
        .header("User-Agent", "Mozilla/5.0")
        .body(hBody::empty())?;

    // 发送请求并等待响应
    let response = client.request(request).await?;
    if response.status() == 404 {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            "图片不存在",
        )));
    }
    // 从响应中提取字节流
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    // 使用image库解析字节流中的图像
    let img = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?;

    Ok(img)
}

async fn get_theme_color(img: DynamicImage) -> String {
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
