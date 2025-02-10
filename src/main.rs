use image::{ImageBuffer, Rgb};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;
use std::f64::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 800;
const NUM_SNOWFLAKES: usize = 300;
const PIXEL_SIZE: i32 = 8;  // Doubled from 4 to 8

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
    
    // Shadow (simple rectangle)
    let shadow_offset = (penguin.z * 2.0) as i32;
    draw_rect(img, base_x + shadow_offset, base_y + shadow_offset, size, size/2, Rgb([0, 0, 0]));
    
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

fn draw_sky_gradient(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) {
    for y in 0..HEIGHT {
        // Calculate gradient color based on y position
        let progress = y as f64 / HEIGHT as f64;
        let r = (100.0 + (130.0 * (1.0 - progress))) as u8; // Darker blue at top
        let g = (150.0 + (80.0 * (1.0 - progress))) as u8;
        let b = (200.0 + (55.0 * (1.0 - progress))) as u8;
        
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

fn main() {
    let mut img = ImageBuffer::new(WIDTH, HEIGHT);
    
    // Get current time as seed
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    
    // Draw sky gradient
    draw_sky_gradient(&mut img);
    
    // Generate and draw snowflakes
    let snowflakes = generate_snowflakes(&mut rng);
    for snowflake in snowflakes.iter() {
        draw_snowflake(&mut img, snowflake);
    }
    
    // Generate random penguins with depth
    let mut penguins: Vec<Penguin> = Vec::new();
    
    // Randomly determine number of penguins (1-5)
    let num_penguins = rng.gen_range(1..=5);
    
    // Create sections for penguin placement
    let section_width = WIDTH / num_penguins as u32;
    
    for i in 0..num_penguins {
        let z = rng.gen_range(0.0..1.0);  // Random depth
        let size = rng.gen_range(80..200); // Size range for prominent penguins
        
        // Calculate y position based on depth
        let y_range_start = (HEIGHT as f64 * 0.4 + z * HEIGHT as f64 * 0.2) as u32;
        let y_range_end = HEIGHT - size - (HEIGHT / 8);
        let y = rng.gen_range(y_range_start..y_range_end);
        
        // Calculate x position within the section
        let section_start = section_width * i as u32;
        let section_end = section_width * (i + 1) as u32;
        let margin = size; // Keep margin equal to penguin size
        
        // Ensure penguin stays within its section while accounting for its size
        let x_min = section_start.saturating_add(margin);
        let x_max = section_end.saturating_sub(margin);
        
        // If section is too narrow, center the penguin
        let x = if x_max <= x_min {
            (section_start + section_end) / 2
        } else {
            rng.gen_range(x_min..=x_max)
        };
        
        penguins.push(Penguin {
            x,
            y,
            z,
            size,
            color: generate_random_color(&mut rng),
            belly_color: Rgb([
                rng.gen_range(200..=255),
                rng.gen_range(200..=255),
                rng.gen_range(200..=255),
            ]),
            rotation: rng.gen_range(0.0..2.0 * PI),
            knife_hand: rng.gen_bool(0.5),
        });
    }
    
    // Sort penguins by depth (back to front)
    penguins.sort_by(|a, b| b.z.partial_cmp(&a.z).unwrap());
    
    // Draw all penguins
    for penguin in penguins.iter() {
        draw_penguin(&mut img, penguin);
    }

    // Add ground/snow effect at the bottom with more pronounced shadows
    let snow_height = HEIGHT / 6;  // Reduced from HEIGHT/4 to HEIGHT/6
    for y in (HEIGHT - snow_height)..HEIGHT {
        let progress = (y - (HEIGHT - snow_height)) as f64 / snow_height as f64;
        let brightness = 235 - (progress * 40.0) as u8;  // Reduced from 255 to 235, increased darkening
        for x in 0..WIDTH {
            let noise = rng.gen_range(-10..=10);  // Reduced noise range from ±15 to ±10
            let color = brightness.saturating_add(noise as u8);
            img.put_pixel(x, y, Rgb([color, color, color]));
        }
    }

    img.save("penguin_art.png").unwrap();
    println!("Generated penguin art has been saved as 'penguin_art.png'");
}
