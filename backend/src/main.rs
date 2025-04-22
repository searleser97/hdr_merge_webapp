use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer, Result};
use actix_cors::Cors;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use uuid::Uuid;

#[derive(Serialize)]
struct HdrResponse {
    message: String,
    download_url: String,
}

async fn upload_images(mut payload: Multipart) -> Result<HttpResponse, Error> {
    // Create a storage directory if it doesn't exist
    let storage_dir = PathBuf::from("./storage");
    tokio::fs::create_dir_all(&storage_dir).await?;
    
    // Create a temporary directory for uploaded files
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_owned();
    
    // Create vectors to store file paths
    let mut file_paths = Vec::new();
    let output_id = Uuid::new_v4().to_string();
    let output_file = format!("{}", output_id);
    let output_path = storage_dir.join(&output_file);
    println!("Output file path: {:?}", output_path);
    
    // Process uploaded files
    while let Ok(Some(mut field)) = payload.try_next().await {
        let filename = field
            .content_disposition()
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), |f| f.to_string());
        
        // Check if file has ORF extension
        if !filename.to_lowercase().ends_with(".orf") {
            continue;
        }
        
        let filepath = temp_path.join(&filename);
        let mut f = tokio::fs::File::create(&filepath).await?;
        
        // Write file content
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            tokio::io::AsyncWriteExt::write_all(&mut f, &data).await?;
        }
        
        file_paths.push(filepath);
    }
    
    // If no ORF files were uploaded, return an error
    if file_paths.is_empty() {
        return Ok(HttpResponse::BadRequest().json("No ORF files uploaded"));
    }
    
    // Execute the hdrmerge command
    let status = Command::new("hdrmerge")
        .args(file_paths.iter().map(|p| p.to_str().unwrap()))
        .args(&["-o", output_path.to_str().unwrap(), "-b", "32"])
        .status()
        .expect("Failed to execute hdrmerge");
    
    // Check if command executed successfully
    if !status.success() {
        return Ok(HttpResponse::InternalServerError().json("Failed to merge HDR images"));
    }
    
    // Return success with download URL
    Ok(HttpResponse::Ok().json(HdrResponse {
        message: format!("Successfully merged {} images", file_paths.len()),
        download_url: format!("/download/{}", output_id),
    }))
}

async fn download_image(path: web::Path<String>) -> Result<NamedFile> {
    let file_id = path.into_inner();
    let path = PathBuf::from("./storage").join(format!("{}.dng", file_id));
    
    Ok(NamedFile::open(path)?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()  // In production, you might want to restrict this
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(web::resource("/upload").route(web::post().to(upload_images)))
            .service(web::resource("/download/{file_id}").route(web::get().to(download_image)))
    })
    .bind("100.90.241.174:8080")?
    .run()
    .await
}
