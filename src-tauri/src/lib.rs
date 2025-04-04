use std::{cmp::Ordering, env, fs, io::Cursor, path::PathBuf};

use average_color::get_averages_colors;
use base64::{engine::general_purpose, Engine};
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat, Pixel, Rgba};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tauri::{async_runtime::RwLock, Manager, State};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            select_image,
            reload_image,
            select_library,
            reload_library,
            export_image,
            refresh,
            get_config,
            set_config,
        ])
        .setup(|app| {
            app.manage(RwLock::new(Store::default()));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn img_as_base64(image: &DynamicImage) -> Result<String, Box<dyn std::error::Error>> {
    // Create a buffer to store the PNG-encoded bytes
    let mut buffer = Vec::new();

    // Write the image as PNG into the buffer
    image.write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)?;

    // Encode the buffer to Base64
    let base64_string = general_purpose::STANDARD.encode(buffer);

    // Return the Base64 string
    Ok(base64_string)
}

#[derive(ts_rs::TS, Clone, Debug, Serialize, Deserialize)]
#[ts(export)]
struct Config {
    pub intermediate_width: usize,
    pub intermediate_height: usize,
    pub prioritize_unique: bool,
    pub unique_threshold: usize,
    pub subpixel_size: usize,
    pub input_path: Option<PathBuf>,
    pub library_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            intermediate_width: 32,
            intermediate_height: 32,
            prioritize_unique: true,
            unique_threshold: 100,
            subpixel_size: 16,
            input_path: None,
            library_path: None,
        }
    }
}

#[derive(Clone, Debug)]
struct Resource {
    pub avg: RGB,
    pub path: PathBuf,
    pub index: usize,
    pub img: Option<DynamicImage>,
}

impl PartialEq for Resource {
    fn eq(&self, other: &Self) -> bool {
        self.avg == other.avg && self.path == other.path
    }
}

impl Eq for Resource {}

impl PartialOrd for Resource {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.avg.partial_cmp(&other.avg) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.path.partial_cmp(&other.path)
    }
}

impl Ord for Resource {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = format!("{:03}{:03}{:03}", self.avg.0, self.avg.1, self.avg.2);
        let b = format!("{:03}{:03}{:03}", other.avg.0, other.avg.1, other.avg.2);
        if a == b {
            Ordering::Equal
        } else if a > b {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}

#[derive(Debug, Default)]
struct Store {
    pub library: Vec<Resource>,
    pub input_img: Option<DynamicImage>,
    pub inter_img: Option<DynamicImage>,
    pub output_img: Option<DynamicImage>,
    pub config: Config,
}

type SyncStore<'a> = State<'a, RwLock<Store>>;
type RGB = (u8, u8, u8);

#[tauri::command]
async fn export_image(store: SyncStore<'_>) -> Result<(), String> {
    println!("Exporting image...");
    let path = FileDialog::new().set_file_name("export.png").save_file().ok_or("Path doesn't exist")?;
    let store = store.write().await;
    store.output_img.as_ref().unwrap().save_with_format(path, ImageFormat::Png).map_err(|_| "Failed to save output image")?;

    Ok(())
}

#[tauri::command]
async fn select_image(store: SyncStore<'_>) -> Result<String, String> {
    println!("Selecting image...");
    let path = FileDialog::new().pick_file().ok_or("Path doesn't exist")?;
    let mut store = store.write().await;

    store.config.input_path = Some(path);

    // Load the image
    store.input_img = Some(
        image::open(store.config.input_path.as_ref().unwrap())
            .map_err(|_| "Failed to open input image")?,
    );

    // Crop the image
    store.inter_img = Some(
        store
            .input_img
            .as_ref()
            .ok_or("Input image does not exist")?
            .resize_to_fill(
                store.config.intermediate_width as u32,
                store.config.intermediate_height as u32,
                image::imageops::FilterType::Gaussian,
            ),
    );

    println!("Successfully selected image {:?}", store.config.input_path);
    let w = store.input_img.as_ref().unwrap().width();
    let h = store.input_img.as_ref().unwrap().height();
    img_as_base64(
        store
            .input_img
            .as_ref()
            .ok_or("Input image does not exist")?,
    )
    .map_err(|err| err.to_string()).map(|f| format!("{:09}x{:09} {}", w, h, f))
}

#[tauri::command]
async fn reload_image(store: SyncStore<'_>) -> Result<String, String> {
    println!("Reloading image...");
    let mut store = store.write().await;

    // Crop the image
    store.inter_img = Some(
        store
            .input_img
            .as_ref()
            .ok_or("Input image does not exist")?
            .resize_to_fill(
                store.config.intermediate_width as u32,
                store.config.intermediate_height as u32,
                image::imageops::FilterType::Gaussian,
            ),
    );

    println!("Successfully reloaded image.");

    let w = store.inter_img.as_ref().unwrap().width();
    let h = store.inter_img.as_ref().unwrap().height();
    img_as_base64(
        store
            .inter_img
            .as_ref()
            .ok_or("Intermediate image does not exist")?,
    )
    .map_err(|err| err.to_string()).map(|f| format!("{:09}x{:09} {}", w, h, f))
}

#[tauri::command]
async fn select_library(store: SyncStore<'_>) -> Result<(), String> {
    println!("Selecting library...");
    let path = FileDialog::new().pick_folder();
    if path.is_none() {
        return Ok(());
    }
    let mut store = store.write().await;
    store.config.library_path = path;

    println!(
        "Successfully selected library {:?}",
        store.config.library_path
    );
    Ok(())
}

#[tauri::command]
async fn reload_library(store: SyncStore<'_>) -> Result<(), String> {
    
    let mut store = store.write().await;
    println!("Reloading library {:?}...", store.config.library_path);

    store.library.clear();
    let entries = fs::read_dir(store.config.library_path.as_ref().unwrap()).unwrap();
    for entry in entries {
        let entry = entry.ok().unwrap();
        if entry.path().is_file() {
            let path = entry.path();
            println!("Processing {path:?}...");
            let colors = get_averages_colors(&[path.to_string_lossy().to_string()]).await;
            if let Some(avg_color) = colors.first() {
                if let Ok(avg_color) = avg_color {
                    if let Some(avg_color) = avg_color {
                        let img = image::open(&path)
                            .map_err(|_| "Failed to open input image")?;

                        let img = img.resize_to_fill(
                            store.config.subpixel_size as u32,
                            store.config.subpixel_size as u32,
                            image::imageops::FilterType::Gaussian,
                        );

                        let len = store.library.len();

                        store.library.push(Resource {
                            avg: (avg_color.r, avg_color.g, avg_color.b),
                            path,
                            index: len,
                            img: Some(img),
                        });
                    }
                }
            }
        }
    }

    println!("Successfully reloaded library...");
    Ok(())
}

#[tauri::command]
async fn refresh(store: SyncStore<'_>) -> Result<String, String> {
    println!("Refreshing output image...");
    let mut store = store.write().await;

    let inter_img = store.inter_img.as_ref().unwrap();

    // Get image dimensions
    let (width, height) = inter_img.dimensions();

    println!("Parsing intermediate image...");
    let unique_threshold = store.config.unique_threshold;
    let prioritize_unique = store.config.prioritize_unique;
    let mut options = store.library.clone();
    let mut results = vec![];

    // Iterate over all pixels
    for y in 0..height {
        println!("Progress: {}%", (y as f32 / height as f32) * 100.0);
        for x in 0..width {
            // Get the RGBA values of the pixel
            let pixel = inter_img.get_pixel(x, y);
            let rgba = pixel.channels();
            let r = rgba[0];
            let g = rgba[1];
            let b = rgba[2];
            let a = rgba[3];
            if a > 125 {
                let path = if prioritize_unique {
                    find(
                        0,
                        true,
                        unique_threshold as f32,
                        &(r, g, b),
                        &store.library,
                        &mut options,
                    )
                } else {
                    find(
                        0,
                        false,
                        unique_threshold as f32,
                        &(r, g, b),
                        &store.library,
                        &mut options,
                    )
                };

                results.push(path);
            } else {
                results.push(None);
            }
        }
    }

    // Dimensions for cropped images
    let subpixel_size = store.config.subpixel_size;

    // Determine the dimensions of the final combined image
    let columns = width as usize; // Number of images per row
    let rows = height as usize; // Calculate rows based on total images
    let combined_width = columns * subpixel_size;
    let combined_height = rows * subpixel_size;

    // Create a blank image buffer for the combined image
    let mut combined_image = ImageBuffer::new(combined_width as u32, combined_height as u32);

    println!("Creating output image...");
    for (index, path) in results.iter().enumerate() {
        println!(
            "Progress: {}%",
            (index as f32 / results.len() as f32) * 100.0
        );
        // Calculate position in the combined image
        let x_offset = (index % columns as usize) * subpixel_size;
        let y_offset = (index / columns as usize) * subpixel_size;

        if let Some(path) = path {
            let res = store.library.get(*path).unwrap();
            let img = res.img.as_ref().unwrap();

            // Paste the cropped image into the combined image
            for y in 0..subpixel_size {
                for x in 0..subpixel_size {
                    let pixel = img.get_pixel(x as u32, y as u32);
                    combined_image.put_pixel((x_offset + x) as u32, (y_offset + y) as u32, pixel);
                }
            }
        } else {
            for y in 0..subpixel_size {
                for x in 0..subpixel_size {
                    let pixel = Rgba([0, 0, 0, 0]);
                    combined_image.put_pixel((x_offset + x) as u32, (y_offset + y) as u32, pixel);
                }
            }
        }
    }

    store.output_img = Some(combined_image.into());

    let w = store.output_img.as_ref().unwrap().width();
    let h = store.output_img.as_ref().unwrap().height();
    println!("Sucessfully refreshed output image...");
    img_as_base64(
        store
            .output_img
            .as_ref()
            .ok_or("Output image does not exist")?,
    )
    .map_err(|err| err.to_string()).map(|f| format!("{:09}x{:09} {}", w, h, f))

    
}

#[tauri::command]
async fn get_config(store: SyncStore<'_>) -> Result<Config, String> {
    println!("Getting config...");
    let store = store.read().await;
    let config = &store.config;

    println!("Successfully got config...");
    Ok(config.clone())
}

#[tauri::command]
async fn set_config(store: SyncStore<'_>, new_config: Config) -> Result<(), String> {
    println!("Setting config...");
    let mut store = store.write().await;
    
    println!("Old config: {:?}\n\nNew config: {:?}", store.config, new_config);

    let config = &mut store.config;

    *config = new_config;

    println!("Successfully set config...");
    Ok(())
}

fn find(
    layer: usize,
    unique: bool,
    threshold: f32,
    target: &RGB,
    resources: &Vec<Resource>,
    options: &mut Vec<Resource>,
) -> Option<usize> {
    if layer > 1 {
        return None;
    }

    let mut result = None;
    let mut result_i = None;
    let mut remove_i = None;
    let mut value = threshold;

    if options.len() == 0 {
        *options = resources.clone();
    }

    for (mid, option) in options.iter().enumerate() {
        let color_distance = calculate_color_distance(&option.avg, target);
        if color_distance < value {
            value = color_distance;
            result = Some(option.path.clone());
            result_i = Some(option.index.clone());
            remove_i = Some(mid);
        }
    }

    if result.is_some() {
        if unique {
            options.remove(remove_i.unwrap());
        }
    } else {
        *options = resources.clone();
        result_i = find(layer + 1, unique, threshold, target, resources, options);
    }

    result_i.map(|f| f.clone())
}

fn calculate_color_distance(color: &RGB, target: &RGB) -> f32 {
    let dr = color.0 as f32 - target.0 as f32;
    let dg = color.1 as f32 - target.1 as f32;
    let db = color.2 as f32 - target.2 as f32;
    (dr.powi(2) + dg.powi(2) + db.powi(2)).sqrt()
}
