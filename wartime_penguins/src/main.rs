use image::{ImageBuffer, Rgb, RgbImage};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::File;
use gif::{Frame, Encoder, Repeat};
use std::collections::HashMap;
use json::{object, JsonValue};
use std::env;
use hyper::Client;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri, request::ApiRequest};
use std::io::Cursor;
use hex;
use tiny_keccak::{Hasher, Keccak};
use serde_json::json;
use ethabi::{Token, encode};
use ciborium;
use cid::Cid;
use multihash::{Code, MultihashDigest};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 800;
const NUM_SNOWFLAKES: usize = 300;
const PIXEL_SIZE: i32 = 8;  // Doubled from 4 to 8
const NUM_FRAMES: u32 = 100;  // Increased from 50 to 100 frames
const MOVEMENT_SPEED: f64 = 0.01;  // Reduced from 0.02 to 0.01 for smoother motion

#[derive(Clone)]
struct Penguin {
    x: u32,
    y: u32,
    z: f64,  // Depth value between 0.0 (front) and 1.0 (back)
    size: u32,
    color: Rgb<u8>,
    belly_color: Rgb<u8>,
    rotation: f64,
    knife_hand: bool, // true for right hand, false for left hand
}

// Quantize colors to create an 8-bit palette effect
fn quantize_color(color: Rgb<u8>) -> Rgb<u8> {
    // Limit colors to 4 levels per channel (64 total colors)
    Rgb([
        (color[0] / 64) * 64,
        (color[1] / 64) * 64,
        (color[2] / 64) * 64,
    ])
}

// Draw a single "pixel" (which is actually a square of PIXEL_SIZE)
fn draw_pixel(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, x: i32, y: i32, color: Rgb<u8>) {
    for dy in 0..PIXEL_SIZE {
        for dx in 0..PIXEL_SIZE {
            let px = x * PIXEL_SIZE + dx;
            let py = y * PIXEL_SIZE + dy;
            if px >= 0 && px < WIDTH as i32 && py >= 0 && py < HEIGHT as i32 {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

// Draw a filled rectangle using our pixel size
fn draw_rect(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, x: i32, y: i32, width: i32, height: i32, color: Rgb<u8>) {
    for dy in 0..height {
        for dx in 0..width {
            draw_pixel(img, x + dx, y + dy, color);
        }
    }
}

fn draw_penguin(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, penguin: &Penguin) {
    let base_x = (penguin.x as i32) / PIXEL_SIZE;
    let base_y = (penguin.y as i32) / PIXEL_SIZE;
    let size = ((penguin.size as f64 * (1.0 - penguin.z * 0.3)) as i32) / PIXEL_SIZE;

    // Quantize colors and adjust for depth
    let depth_factor = 1.0 - penguin.z * 0.3;
    let color = quantize_color(Rgb([
        ((penguin.color[0] as f64) * depth_factor) as u8,
        ((penguin.color[1] as f64) * depth_factor) as u8,
        ((penguin.color[2] as f64) * depth_factor) as u8,
    ]));

    // Draw short legs (always 3 pixels tall)
    let leg_height = 3;
    let leg_width = 2;
    let leg_spacing = size / 3;

    // Left leg
    draw_rect(img,
        base_x + leg_spacing - leg_width/2,
        base_y + size,
        leg_width,
        leg_height,
        color
    );

    // Right leg
    draw_rect(img,
        base_x + size - leg_spacing - leg_width/2,
        base_y + size,
        leg_width,
        leg_height,
        color
    );

    // Body (rectangular)
    draw_rect(img, base_x, base_y, size, size, color);

    // Belly (rectangular, lighter color)
    let belly_width = (size as f64 * 0.7) as i32;
    let belly_x = base_x + (size - belly_width) / 2;
    let belly_color = quantize_color(Rgb([
        ((penguin.belly_color[0] as f64) * depth_factor) as u8,
        ((penguin.belly_color[1] as f64) * depth_factor) as u8,
        ((penguin.belly_color[2] as f64) * depth_factor) as u8,
    ]));
    draw_rect(img, belly_x, base_y + size/3, belly_width, size/2, belly_color);

    // Head (square)
    let head_size = (size as f64 * 0.6) as i32;
    draw_rect(img,
        base_x + (size - head_size)/2,
        base_y - head_size/2,
        head_size,
        head_size,
        color
    );

    // Eyes (2x2 pixels with highlight)
    let eye_spacing = head_size / 3;
    draw_pixel(img, base_x + size/2 - eye_spacing, base_y - head_size/4, Rgb([0, 0, 0]));
    draw_pixel(img, base_x + size/2 + eye_spacing - 1, base_y - head_size/4, Rgb([0, 0, 0]));
    draw_pixel(img, base_x + size/2 - eye_spacing, base_y - head_size/4 - 1, Rgb([255, 255, 255]));
    draw_pixel(img, base_x + size/2 + eye_spacing - 1, base_y - head_size/4 - 1, Rgb([255, 255, 255]));

    // Beak (pixel triangle)
    let beak_color = quantize_color(Rgb([255, 165, 0]));
    for i in 0..3 {
        draw_pixel(img, base_x + size/2 - 1 + i, base_y - head_size/4 + 1, beak_color);
    }
    draw_pixel(img, base_x + size/2, base_y - head_size/4 + 2, beak_color);

    // Flippers (rectangles)
    let flipper_width = 2;
    let flipper_height = size/3;

    // Left flipper
    draw_rect(img, base_x - flipper_width, base_y + size/3, flipper_width, flipper_height, color);

    // Right flipper
    draw_rect(img, base_x + size, base_y + size/3, flipper_width, flipper_height, color);

    // Knife (pixelated)
    if penguin.knife_hand {
        // Right hand knife (vertical)
        let handle_length = 8;
        let blade_length = 14;
        // Handle (vertical)
        draw_rect(img, base_x + size + flipper_width, base_y + size/3 - handle_length, 3, handle_length, Rgb([139, 69, 19]));
        // Blade (vertical, pointing up)
        draw_rect(img, base_x + size + flipper_width, base_y + size/3 - handle_length - blade_length, 3, blade_length, Rgb([192, 192, 192]));
        // Blade point
        draw_rect(img, base_x + size + flipper_width, base_y + size/3 - handle_length - blade_length - 2, 3, 2, Rgb([192, 192, 192]));
        // Guard
        draw_rect(img, base_x + size + flipper_width - 2, base_y + size/3 - handle_length, 7, 2, Rgb([139, 69, 19]));
    } else {
        // Left hand knife (vertical)
        let handle_length = 8;
        let blade_length = 14;
        // Handle (vertical)
        draw_rect(img, base_x - flipper_width - 3, base_y + size/3 - handle_length, 3, handle_length, Rgb([139, 69, 19]));
        // Blade (vertical, pointing up)
        draw_rect(img, base_x - flipper_width - 3, base_y + size/3 - handle_length - blade_length, 3, blade_length, Rgb([192, 192, 192]));
        // Blade point
        draw_rect(img, base_x - flipper_width - 3, base_y + size/3 - handle_length - blade_length - 2, 3, 2, Rgb([192, 192, 192]));
        // Guard
        draw_rect(img, base_x - flipper_width - 5, base_y + size/3 - handle_length, 7, 2, Rgb([139, 69, 19]));
    }
}

fn generate_random_color(rng: &mut ChaCha8Rng) -> Rgb<u8> {
    Rgb([
        rng.gen_range(50..220),
        rng.gen_range(50..220),
        rng.gen_range(50..220),
    ])
}

enum SkyTheme {
    Day,
    Dawn,
    Dusk,
    Night,
    Aurora,
}

fn get_random_sky_theme(rng: &mut ChaCha8Rng) -> SkyTheme {
    match rng.gen_range(0..5) {
        0 => SkyTheme::Day,
        1 => SkyTheme::Dawn,
        2 => SkyTheme::Dusk,
        3 => SkyTheme::Night,
        _ => SkyTheme::Aurora,
    }
}

fn draw_sky_gradient(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, theme: &SkyTheme) {
    let (top_color, bottom_color) = match theme {
        SkyTheme::Day => (
            (100, 150, 255),   // Light blue top
            (180, 220, 255)    // Lighter blue bottom
        ),
        SkyTheme::Dawn => (
            (70, 100, 150),    // Dark blue top
            (255, 180, 150)    // Pink/orange bottom
        ),
        SkyTheme::Dusk => (
            (60, 80, 120),     // Dark blue top
            (255, 140, 100)    // Orange/red bottom
        ),
        SkyTheme::Night => (
            (10, 20, 40),      // Very dark blue top
            (40, 50, 80)       // Slightly lighter blue bottom
        ),
        SkyTheme::Aurora => (
            (20, 40, 60),      // Dark blue-green top
            (40, 180, 120)     // Green-teal bottom
        ),
    };

    for y in 0..HEIGHT {
        let progress = y as f64 / HEIGHT as f64;

        // Interpolate between top and bottom colors
        let r = (top_color.0 as f64 * (1.0 - progress) + bottom_color.0 as f64 * progress) as u8;
        let g = (top_color.1 as f64 * (1.0 - progress) + bottom_color.1 as f64 * progress) as u8;
        let b = (top_color.2 as f64 * (1.0 - progress) + bottom_color.2 as f64 * progress) as u8;

        for x in 0..WIDTH {
            img.put_pixel(x, y, Rgb([r, g, b]));
        }
    }
}

struct Snowflake {
    x: f64,
    y: f64,
    size: f64,
    sparkle: bool,
}

fn generate_snowflakes(rng: &mut ChaCha8Rng) -> Vec<Snowflake> {
    (0..NUM_SNOWFLAKES)
        .map(|_| Snowflake {
            x: rng.gen_range(0.0..WIDTH as f64),
            y: rng.gen_range(0.0..HEIGHT as f64),
            size: rng.gen_range(2.0..5.0),
            sparkle: rng.gen_bool(0.3), // 30% chance of sparkle effect
        })
        .collect()
}

fn draw_snowflake(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, snowflake: &Snowflake) {
    let x = (snowflake.x as i32) / PIXEL_SIZE;
    let y = (snowflake.y as i32) / PIXEL_SIZE;

    // Simple pixel for snow
    draw_pixel(img, x, y, Rgb([255, 255, 255]));
    if snowflake.sparkle {
        // Cross pattern for sparkle
        draw_pixel(img, x, y - 1, Rgb([255, 255, 255]));
        draw_pixel(img, x, y + 1, Rgb([255, 255, 255]));
        draw_pixel(img, x - 1, y, Rgb([255, 255, 255]));
        draw_pixel(img, x + 1, y, Rgb([255, 255, 255]));
    }
}

fn draw_ground(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) {
    let horizon_y = (HEIGHT as f64 * 0.4) as u32;

    for y in horizon_y..HEIGHT {
        // Calculate progress from horizon to bottom
        let progress = (y - horizon_y) as f64 / (HEIGHT - horizon_y) as f64;

        // Create a snowy ground effect that gets darker towards the horizon
        let base_value = 255.0 - (40.0 * (1.0 - progress));
        let color = Rgb([
            base_value as u8,
            base_value as u8,
            base_value as u8,
        ]);

        for x in 0..WIDTH {
            img.put_pixel(x, y, color);
        }
    }
}

fn update_penguin_position(penguin: &mut Penguin) {
    // Move penguin forward (decrease z)
    penguin.z -= MOVEMENT_SPEED;

    // If penguin gets too close, reset to back
    if penguin.z < 0.0 {
        penguin.z = 1.0;
    }

    // Update y position based on new z
    let horizon_y = HEIGHT as f64 * 0.4;
    let y_offset = penguin.z * horizon_y * 0.3;
    let y_range_start = (horizon_y + y_offset) as u32;
    let y_range_end = HEIGHT - penguin.size - (HEIGHT / 8);
    penguin.y = y_range_start + (y_range_end - y_range_start) / 2;
}

fn create_frame(img: &RgbImage) -> Frame<'static> {
    // Convert RGB image to indexed colors (required for GIF)
    let mut pixels = Vec::new();
    let mut palette = vec![0u8; 768]; // 256 RGB colors
    let mut color_map = HashMap::new();
    let mut next_color_index = 0;

    // Initialize with some basic colors we know we'll use
    // White (for snow)
    palette[0] = 255; palette[1] = 255; palette[2] = 255;
    // Black (for eyes)
    palette[3] = 0; palette[4] = 0; palette[5] = 0;
    // Brown (for knife handle)
    palette[6] = 139; palette[7] = 69; palette[8] = 19;
    // Silver (for knife blade)
    palette[9] = 192; palette[10] = 192; palette[11] = 192;
    next_color_index = 4;

    // Convert image to indexed colors
    for pixel in img.pixels() {
        let key = (pixel[0], pixel[1], pixel[2]);
        let color_index = if let Some(&idx) = color_map.get(&key) {
            idx
        } else {
            if next_color_index < 256 {
                let idx = next_color_index;
                palette[idx * 3] = key.0;
                palette[idx * 3 + 1] = key.1;
                palette[idx * 3 + 2] = key.2;
                color_map.insert(key, idx);
                next_color_index += 1;
                idx
            } else {
                // If we run out of colors, use the closest existing one
                0 // Default to first color if we run out
            }
        };
        pixels.push(color_index as u8);
    }

    // Trim palette to actually used colors
    palette.truncate(next_color_index * 3);

    Frame {
        width: WIDTH as u16,
        height: HEIGHT as u16,
        buffer: pixels.into(),
        delay: 2,  // Reduced from 5 to 2 (2/100ths of a second) for smoother animation
        transparent: None,
        needs_user_input: false,
        top: 0,
        left: 0,
        dispose: gif::DisposalMethod::Keep,
        interlaced: false,
        palette: Some(palette),
    }
}

// Add this helper function to convert RGB to hex color string
fn rgb_to_hex(color: &Rgb<u8>) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

async fn emit_image_keccak256(
    client: &hyper::Client<hyper::client::HttpConnector>,
    server_addr: &str,
    data: Vec<u8>,
) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    // Calculate Keccak-256 hash
    let mut keccak = Keccak::v256();
    let mut hash = [0u8; 32];
    keccak.update(&data);
    keccak.finalize(&mut hash);

    // Create GIO payload
    let hex_data = format!("0x{}", hex::encode(&data));
    let gio = object! {
        "domain" => 0x2c,
        "id" => hex_data
    };

    // Send GIO request
    let gio_request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("{}/gio", server_addr))
        .header("Content-Type", "application/json")
        .body(hyper::Body::from(gio.dump()))?;

    let response = client.request(gio_request).await?;
    
    if response.status() == hyper::StatusCode::ACCEPTED {
        println!("Image Keccak256 GIO emitted successfully");
    } else {
        eprintln!("Failed to emit Image Keccak256 GIO. Status: {}", response.status());
    }

    Ok(hash)
}

async fn emit_notice(
    client: &hyper::Client<hyper::client::HttpConnector>,
    server_addr: &str,
    data: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let hex_string = format!("0x{}", hex::encode(&data));
    let notice = object! { "payload" => hex_string };
    let notice_request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("{}/notice", server_addr))
        .header("Content-Type", "application/json")
        .body(hyper::Body::from(notice.dump()))?;

    let _response = client.request(notice_request).await?;
    Ok(())
}

async fn ipfs_add_with_keccak(ipfs: &IpfsClient, data: Vec<u8>) -> Result<String, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(data);
    let options = ipfs_api_backend_hyper::request::Add {
        hash: Some("keccak-256".into()),
        cid_version: Some(1),
        ..Default::default()
    };
    let add = ipfs.add_with_options(cursor, options).await?;
    Ok(add.hash)
}

async fn generate_nft_metadata(
    client: &hyper::Client<hyper::client::HttpConnector>,
    server_addr: &str,
    ipfs_hash: &str,
    num_penguins: usize,
    sky_theme: &SkyTheme,
    penguins: &[Penguin],
) -> Result<String, Box<dyn std::error::Error>> {
    // Create attributes for each penguin
    let mut penguin_attributes = Vec::new();
    for (i, penguin) in penguins.iter().enumerate() {
        penguin_attributes.push(json!({
            "trait_type": format!("Penguin {} Color", i + 1),
            "value": rgb_to_hex(&penguin.color)
        }));
        penguin_attributes.push(json!({
            "trait_type": format!("Penguin {} Size", i + 1),
            "value": penguin.size
        }));
        penguin_attributes.push(json!({
            "trait_type": format!("Penguin {} Knife Hand", i + 1),
            "value": if penguin.knife_hand { "Right" } else { "Left" }
        }));
    }

    // Create the metadata JSON with all attributes
    let mut metadata = json!({
        "name": format!("{} Wartime Penguin(s)", num_penguins),
        "description": "Penguin warrior in a snowy battlefield",
        "image": format!("ipfs://{}", ipfs_hash),
        "attributes": [
            {
                "trait_type": "Number of Penguins",
                "value": num_penguins
            },
            {
                "trait_type": "Sky Theme",
                "value": match sky_theme {
                    SkyTheme::Day => "Day",
                    SkyTheme::Dawn => "Dawn",
                    SkyTheme::Dusk => "Dusk",
                    SkyTheme::Night => "Night",
                    SkyTheme::Aurora => "Aurora"
                }
            },
        ]
    });

    // Add penguin-specific attributes
    if let Some(attributes) = metadata["attributes"].as_array_mut() {
        attributes.extend(penguin_attributes);
    }

    // Convert metadata to bytes
    let metadata_bytes = metadata.to_string().into_bytes();
    
    // Upload metadata to IPFS with keccak-256 and CIDv1
    let ipfs = IpfsClient::default();
    let metadata_hash = ipfs_add_with_keccak(&ipfs, metadata_bytes).await?;
    println!("Metadata uploaded to IPFS with hash: {}", metadata_hash);
    println!("You can view metadata at: http://127.0.0.1:8080/ipfs/{}", metadata_hash);
    
    Ok(metadata_hash)
}

async fn get_ipfs_refs(ipfs: &IpfsClient, hash: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Create HTTP client
    let client = hyper::Client::new();
    
    // Build request to IPFS refs API with recursive flag
    let request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("http://127.0.0.1:5001/api/v0/refs?arg={}&recursive=true", hash))
        .body(hyper::Body::empty())?;

    // Make request and get response
    let response = client.request(request).await?;
    let body = hyper::body::to_bytes(response).await?;
    let body_str = String::from_utf8(body.to_vec())?;

    // Parse response and collect unique refs
    let mut unique_refs = std::collections::HashSet::new();
    for line in body_str.lines() {
        if let Ok(ref_obj) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(ref_str) = ref_obj["Ref"].as_str() {
                unique_refs.insert(ref_str.to_string());
            }
        }
    }

    // Convert HashSet to Vec, starting with the original hash
    let mut result = vec![hash.to_string()];
    result.extend(unique_refs.into_iter());
    Ok(result)
}

async fn get_ipfs_block(client: &hyper::Client<hyper::client::HttpConnector>, cid: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("http://127.0.0.1:5001/api/v0/block/get?arg={}", cid))
        .body(hyper::Body::empty())?;

    let response = client.request(request).await?;
    let body = hyper::body::to_bytes(response).await?;
    Ok(body.to_vec())
}

fn verify_keccak_cid(cid_str: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let cid = Cid::try_from(cid_str)?;
    let mh = cid.hash();
    println!("CID: {:?}", cid);
    println!("MH: {:?}", mh.code());   
    Ok(mh.code() == 0x1b)
}

async fn process_ipfs_blocks(
    client: &hyper::Client<hyper::client::HttpConnector>,
    server_addr: &str,
    ipfs_blocks: &[String],
) -> Result<Vec<[u8; 32]>, Box<dyn std::error::Error>> {
    let mut block_hashes = Vec::new();
    
    for block_cid in ipfs_blocks {
        // Verify CID uses keccak-256
        if !verify_keccak_cid(block_cid)? {
            return Err(format!("Block CID {:?} does not use keccak-256 multihash", block_cid).into());
        }
        
        // Get block data
        let block_data = get_ipfs_block(client, block_cid).await?;
        
        // Calculate and emit keccak256 hash
        let hash = emit_image_keccak256(client, server_addr, block_data).await?;
        block_hashes.push(hash);
    }
    
    Ok(block_hashes)
}

async fn emit_abi_encoded_notice(
    client: &hyper::Client<hyper::client::HttpConnector>,
    server_addr: &str,
    metadata_cid: &str,
    ipfs_blocks: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    // Process all IPFS blocks
    let block_hashes = process_ipfs_blocks(client, server_addr, ipfs_blocks).await?;
    
    // CBOR encode the IPFS block references and their hashes
    let mut cbor_data = Vec::new();
    let block_data: Vec<(&str, &[u8])> = ipfs_blocks.iter()
        .zip(block_hashes.iter())
        .map(|(cid, hash)| (cid.as_str(), hash.as_slice()))
        .collect();
    ciborium::ser::into_writer(&block_data, &mut cbor_data)?;
    
    let hash = emit_image_keccak256(client, server_addr, cbor_data).await?;   
    // Create tokens for ABI encoding with hash
    let tokens = vec![
        Token::String(metadata_cid.to_string()),
        Token::FixedBytes(hash.to_vec()),
    ];
    
    // Encode the tokens
    let encoded = encode(&tokens);
    
    // Create and emit notice
    let hex_string = format!("0x{}", hex::encode(&encoded));
    let notice = object! { "payload" => hex_string };
    let notice_request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("{}/notice", server_addr))
        .header("Content-Type", "application/json")
        .body(hyper::Body::from(notice.dump()))?;

    let _response = client.request(notice_request).await?;
    println!("ABI encoded notice emitted successfully");
    Ok(())
}

pub async fn generate_penguin_gif(
    client: &hyper::Client<hyper::client::HttpConnector>,
    server_addr: &str,
    seed: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup GIF encoder with global color table
    let mut image_file = File::create("/tmp/penguin_rush.gif")?;
    let mut encoder = Encoder::new(&mut image_file, WIDTH as u16, HEIGHT as u16, &[])?;
    encoder.set_repeat(Repeat::Infinite)?;

    // Initialize RNG with provided seed
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // Get random sky theme (stays constant through animation)
    let sky_theme = get_random_sky_theme(&mut rng);

    // Generate initial penguins with their permanent colors
    let mut penguins: Vec<Penguin> = Vec::new();
    let num_penguins = rng.gen_range(2..=6);
    let section_width = WIDTH / num_penguins as u32;

    // Pre-generate colors for each penguin
    let penguin_colors: Vec<Rgb<u8>> = (0..num_penguins)
        .map(|_| generate_random_color(&mut rng))
        .collect();

    for (i, &color) in (0..num_penguins).zip(penguin_colors.iter()) {
        let z = rng.gen_range(0.0..1.0);
        let size = rng.gen_range(80..160);

        let section_start = section_width * i as u32;
        let section_end = section_width * (i + 1) as u32;
        let margin = size / 2;

        let x_min = section_start.saturating_add(margin);
        let x_max = section_end.saturating_sub(margin);

        let x = if x_max <= x_min {
            (section_start + section_end) / 2
        } else {
            rng.gen_range(x_min..x_max)
        };

        let horizon_y = HEIGHT as f64 * 0.4;
        let y_offset = z * horizon_y * 0.3;
        let y_range_start = (horizon_y + y_offset) as u32;
        let y_range_end = HEIGHT - size - (HEIGHT / 8);
        let y = rng.gen_range(y_range_start..y_range_end);

        penguins.push(Penguin {
            x,
            y,
            z,
            size,
            color, // Use the pre-generated color
            belly_color: Rgb([230, 230, 230]),
            rotation: rng.gen_range(-0.2..0.2),
            knife_hand: rng.gen_bool(0.5),
        });
    }

    // Generate frames
    for _ in 0..NUM_FRAMES {
        let mut img = ImageBuffer::new(WIDTH, HEIGHT);

        // Draw sky
        draw_sky_gradient(&mut img, &sky_theme);

        // Draw ground
        draw_ground(&mut img);

        // Generate and draw snowflakes (new each frame for animation effect)
        let snowflakes = generate_snowflakes(&mut rng);
        for snowflake in snowflakes.iter() {
            draw_snowflake(&mut img, snowflake);
        }

        // Update and sort penguins by depth
        for penguin in penguins.iter_mut() {
            update_penguin_position(penguin);
        }
        penguins.sort_by(|a, b| b.z.partial_cmp(&a.z).unwrap());

        // Draw penguins
        for penguin in penguins.iter() {
            draw_penguin(&mut img, penguin);
        }

        // Add frame to GIF
        encoder.write_frame(&create_frame(&img)).unwrap();
    }

    // After writing the GIF file, upload it to IPFS
    let ipfs = IpfsClient::default();
    let gif_path = "/tmp/penguin_rush.gif";
    
    println!("Uploading to IPFS...");
    let data = std::fs::read(gif_path)?;
    
    // Upload to IPFS with keccak-256 and CIDv1
    let res_hash = ipfs_add_with_keccak(&ipfs, data.clone()).await?;
    println!("Image uploaded to IPFS with hash: {}", res_hash);
    println!("You can view image at: http://127.0.0.1:8080/ipfs/{}", res_hash);
    
    // Get IPFS block references
    let ipfs_blocks = get_ipfs_refs(&ipfs, &res_hash).await?;
    
    // Generate and upload NFT metadata to IPFS
    let metadata_hash = generate_nft_metadata(
        client, 
        server_addr, 
        &res_hash, 
        num_penguins,
        &sky_theme,
        &penguins
    ).await?;
    
    // Emit ABI encoded notice with metadata CID and image hash
    emit_abi_encoded_notice(client, server_addr, &metadata_hash, &ipfs_blocks).await?;
    
    // Also emit the regular hashes notice for logging
    let hashes = format!("Image: {}\nMetadata: {}", res_hash, metadata_hash);
    println!("{}", hashes);
    
    Ok(())
}

pub async fn handle_advance(
    client: &hyper::Client<hyper::client::HttpConnector>,
    server_addr: &str,
    request: JsonValue,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    println!("Received advance request data {}", &request);
    let payload = request["data"]["payload"].as_str().ok_or("Missing payload")?;
    
    // Remove "0x" prefix and decode hex
    let payload_bytes = hex::decode(&payload[2..])?;
    
    // Calculate keccak hash of payload
    let mut keccak = Keccak::v256();
    let mut hash = [0u8; 32];
    keccak.update(&payload_bytes);
    keccak.finalize(&mut hash);
    
    // Use first 8 bytes of hash as seed
    let seed = u64::from_be_bytes(hash[0..8].try_into().unwrap());
    
    generate_penguin_gif(client, server_addr, seed).await?;
    Ok("accept")
}

pub async fn handle_inspect(
    _client: &hyper::Client<hyper::client::HttpConnector>,
    _server_addr: &str,
    request: JsonValue,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    println!("Received inspect request data {}", &request);
    let _payload = request["data"]["payload"].as_str().ok_or("Missing payload")?;
    Ok("accept")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;
    let mut status = "accept";
    loop {
        println!("Sending finish");
        let response = object! {"status" => status};
        let request = hyper::Request::builder()
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .uri(format!("{}/finish", &server_addr))
            .body(hyper::Body::from(response.dump()))?;
        let response = client.request(request).await?;
        println!("Received finish status {}", response.status());
        if response.status() == hyper::StatusCode::ACCEPTED {
            println!("No pending rollup request, trying again");
        } else {
            let body = hyper::body::to_bytes(response).await?;
            let utf = std::str::from_utf8(&body)?;
            let req = json::parse(utf)?;
            let request_type = req["request_type"].as_str().ok_or("request_type is not a string")?;
            status = match request_type {
                "advance_state" => handle_advance(&client, &server_addr[..], req).await?,
                "inspect_state" => handle_inspect(&client, &server_addr[..], req).await?,
                _ => {
                    eprintln!("Unknown request type");
                    "reject"
                }
            };
        }
    }
}
