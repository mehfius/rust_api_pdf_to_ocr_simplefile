# PDF to OCR API

API REST em Rust para converter PDFs em texto usando OCR.

## Dependências

- actix-web = "4.9"
- serde = { version = "1.0", features = ["derive"] }
- serde_json = "1.0"
- image = "0.25.6"
- base64 = "0.21"
- reqwest = { version = "0.11", features = ["json"] }
- mupdf = "0.5.0"
- rusty-tesseract = "1.1.10"

## Funcionalidades

- Endpoint `/pdf_to_ocr` que aceita URLs de PDFs
- Extrai texto de cada página do PDF usando OCR
- Suporte para múltiplas páginas
- Retorna o texto extraído e tempo de processamento por página
- Suporte para OCR em português 