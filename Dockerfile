FROM debian:bullseye

RUN apt-get update && apt-get install -y \
    tesseract-ocr \
    tesseract-ocr-por \
    libssl1.1 \
    ca-certificates \
    && update-ca-certificates \    
    && rm -rf /var/lib/apt/lists/*

COPY target/release/rust_api_pdf_to_ocr_simplefile /usr/local/bin/

EXPOSE 5001

CMD ["rust_api_pdf_to_ocr_simplefile"]