use actix_web::{post, web, App, HttpResponse, HttpServer};
use mupdf::{Document, Matrix, Colorspace};
use serde::Deserialize;
use serde_json::json;
use std::io::Cursor;
use image::{DynamicImage, ImageFormat};
use rusty_tesseract::{Image, Args};
use std::collections::HashMap;
use std::time::Instant;
use rusty_tesseract::image_to_string;

#[derive(Deserialize)]
struct PdfInput {
    url: String,
}

fn clean_ocr_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}

fn extract_text_from_image(image_bytes: &[u8]) -> Result<(String, u128), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    let dyn_img = image::load_from_memory(image_bytes)?;
    let img = Image::from_dynamic_image(&dyn_img)?;
    
    let mut config_variables = HashMap::new();
    config_variables.insert("tessedit_create_txt".to_string(), "1".to_string());
    
    let args = Args {
        lang: "por".to_string(),
        dpi: Some(300),
        psm: Some(6),
        oem: Some(3),
        config_variables,
    };
    
    let texto = image_to_string(&img, &args)?;
    let texto_limpo = clean_ocr_text(&texto);
    let duration_ms = start_time.elapsed().as_millis();
    
    Ok((texto_limpo, duration_ms))
}

#[post("/pdf_to_ocr")]
async fn convert_pdf(input: Option<web::Json<PdfInput>>) -> HttpResponse {
    let input = match input {
        Some(i) => i,
        None => {
            return HttpResponse::BadRequest().json(json!({
                "error": "JSON inválido ou ausente"
            }))
        }
    };

    let download_start_time = Instant::now();
    let response = match reqwest::get(&input.url).await {
        Ok(res) => res,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Falha ao baixar PDF: {}", e)
            }))
        }
    };

    let pdf_data = match response.bytes().await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Erro ao ler resposta: {}", e)
            }))
        }
    };
    let download_time_ms = download_start_time.elapsed().as_millis();
    println!("Tempo de download do PDF: {}ms", download_time_ms);

    let doc = match Document::from_bytes(&pdf_data, "") {
        Ok(doc) => doc,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Não foi possível carregar o PDF: {}", e)
            }))
        }
    };

    let page_count = doc.pages().unwrap().count();
    let mut ocr_results = Vec::new();
    let total_start_time = Instant::now();

    for page_index in 0..page_count {
        let page_start_time = Instant::now();

        let page = match doc.load_page(page_index as i32) {
            Ok(page) => page,
            Err(e) => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Erro ao carregar página {}: {}", page_index + 1, e)
                }))
            }
        };

        let matrix = Matrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);
        let colorspace = Colorspace::device_rgb();

        let image_extraction_start = Instant::now();
        let pixmap = match page.to_pixmap(&matrix, &colorspace, false, false) {
            Ok(pixmap) => pixmap,
            Err(e) => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Erro ao gerar pixmap da página {}: {}", page_index + 1, e)
                }))
            }
        };

        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;
        let samples = pixmap.samples().to_vec();

        let img = match image::RgbImage::from_raw(width, height, samples) {
            Some(img) => DynamicImage::ImageRgb8(img),
            None => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Falha ao criar imagem da página {}", page_index + 1)
                }))
            }
        };

        let mut buffer = Cursor::new(Vec::new());
        if let Err(e) = img.write_to(&mut buffer, ImageFormat::Png) {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Erro ao salvar imagem da página {}: {}", page_index + 1, e)
            }));
        }

        let image_bytes = buffer.into_inner();
        let image_extraction_time_ms = image_extraction_start.elapsed().as_millis();

        let ocr_result = match extract_text_from_image(&image_bytes) {
            Ok((text, ocr_duration_ms)) => {
                println!(
                    "Página {}: Extração de imagem: {}ms, OCR: {}ms",
                    page_index + 1, image_extraction_time_ms, ocr_duration_ms
                );
                json!({
                    "text": text
                })
            }
            Err(e) => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Erro ao processar OCR (página {}): {}", page_index + 1, e)
                }))
            }
        };

        let page_total_time_ms = page_start_time.elapsed().as_millis();
        println!("Página {}: Tempo total de processamento: {}ms", page_index + 1, page_total_time_ms);

        ocr_results.push(json!({
            "page": page_index + 1,
            "ocr_result": ocr_result
        }));
    }

    let total_processing_time_ms = total_start_time.elapsed().as_millis();
    println!("Tempo total de processamento (todas as páginas): {}ms", total_processing_time_ms);

    HttpResponse::Ok().json(json!({
        "results": ocr_results,
        "page_count": page_count
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Servidor rodando em http://0.0.0.0:5001");
    HttpServer::new(|| App::new().service(convert_pdf))
        .bind("0.0.0.0:5001")?
        .run()
        .await
}