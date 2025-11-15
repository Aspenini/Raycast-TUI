use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, size},
};
use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};

const MAP_WIDTH: usize = 24;
const MAP_HEIGHT: usize = 24;
const FOV: f64 = 0.66; // Field of view
const MOVE_SPEED: f64 = 0.05;
const ROTATION_SPEED: f64 = 0.03;

// Map: 1 = wall, 0 = empty space
const MAP: &[&str] = &[
    "111111111111111111111111",
    "100000000011000000000001",
    "100000000011000000000001",
    "100000000011000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "100000000000000000000001",
    "111111111111111111111111",
];

struct Player {
    x: f64,
    y: f64,
    angle: f64,
}

struct Raycaster {
    player: Player,
    last_width: usize,
    last_height: usize,
}

impl Raycaster {
    fn new() -> Self {
        Raycaster {
            player: Player {
                x: 2.0,
                y: 2.0,
                angle: 0.0,
            },
            last_width: 0,
            last_height: 0,
        }
    }

    fn get_map_value(&self, x: usize, y: usize) -> u8 {
        if x < MAP_WIDTH && y < MAP_HEIGHT {
            MAP[y].as_bytes()[x] - b'0'
        } else {
            1
        }
    }

    fn cast_ray(&self, ray_angle: f64) -> f64 {
        let sin = ray_angle.sin();
        let cos = ray_angle.cos();
        
        let x = self.player.x;
        let y = self.player.y;
        
        let delta_x = if cos.abs() < 0.0001 { 1e30 } else { (1.0 / cos).abs() };
        let delta_y = if sin.abs() < 0.0001 { 1e30 } else { (1.0 / sin).abs() };
        
        let step_x = if cos < 0.0 { -1 } else { 1 };
        let step_y = if sin < 0.0 { -1 } else { 1 };
        
        let mut map_x = x.floor() as i32;
        let mut map_y = y.floor() as i32;
        
        let mut side_dist_x = if cos < 0.0 {
            (x - map_x as f64) * delta_x
        } else {
            (map_x as f64 + 1.0 - x) * delta_x
        };
        let mut side_dist_y = if sin < 0.0 {
            (y - map_y as f64) * delta_y
        } else {
            (map_y as f64 + 1.0 - y) * delta_y
        };
        
        let mut hit = false;
        let mut side = false;
        
        while !hit {
            if side_dist_x < side_dist_y {
                side_dist_x += delta_x;
                map_x += step_x;
                side = false;
            } else {
                side_dist_y += delta_y;
                map_y += step_y;
                side = true;
            }
            
            if map_x < 0 || map_x >= MAP_WIDTH as i32 || map_y < 0 || map_y >= MAP_HEIGHT as i32 {
                break;
            }
            
            if self.get_map_value(map_x as usize, map_y as usize) == 1 {
                hit = true;
            }
        }
        
        let perp_wall_dist = if !side {
            side_dist_x - delta_x
        } else {
            side_dist_y - delta_y
        };
        
        perp_wall_dist
    }

    fn render(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        let (screen_width, screen_height) = size()?;
        let screen_width = screen_width as usize;
        let screen_height = screen_height as usize;
        
        // Clear screen if size changed (handles terminal resize)
        if screen_width != self.last_width || screen_height != self.last_height {
            execute!(stdout, Clear(ClearType::All))?;
            self.last_width = screen_width;
            self.last_height = screen_height;
        }
        
        // Build frame buffer with double vertical resolution (2 pixels per character)
        let double_height = screen_height * 2;
        let mut frame_buffer = vec![vec![0u8; screen_width]; double_height];
        
        // Calculate all columns
        for x in 0..screen_width {
            let camera_x = 2.0 * x as f64 / screen_width as f64 - 1.0;
            let ray_angle = self.player.angle + (camera_x * FOV).atan();
            
            let perp_wall_dist = self.cast_ray(ray_angle);
            
            // Use double height for calculations
            let line_height = (double_height as f64 / perp_wall_dist.max(0.1)) as usize;
            let draw_start = ((double_height as i32 - line_height as i32) / 2).max(0);
            let draw_end = ((double_height as i32 + line_height as i32) / 2).min(double_height as i32);
            
            // Get 256-color code for wall based on distance
            let wall_color = self.distance_to_color(perp_wall_dist);
            
            for y in 0..double_height {
                let y_i32 = y as i32;
                if y_i32 >= draw_start && y_i32 < draw_end {
                    frame_buffer[y][x] = wall_color;
                } else if y_i32 < draw_start {
                    // Ceiling - darker gradient based on distance from center
                    let dist_from_center = (draw_start - y_i32) as f64 / double_height as f64;
                    frame_buffer[y][x] = self.ceiling_color(dist_from_center);
                } else {
                    // Floor - darker gradient based on distance from center
                    let dist_from_center = (y_i32 - draw_end) as f64 / double_height as f64;
                    frame_buffer[y][x] = self.floor_color(dist_from_center);
                }
            }
        }
        
        // Build output string using half-block characters for double resolution
        // Use ▀ (upper half) and ▄ (lower half) to get 2 pixels per character
        let mut output = String::with_capacity(screen_width * screen_height * 30);
        output.push_str("\x1b[H"); // Move cursor to home (0,0) without clearing
        
        let mut current_fg = 0u8;
        let mut current_bg = 0u8;
        
        for y in 0..screen_height {
            let upper_y = y * 2;
            let lower_y = y * 2 + 1;
            
            for x in 0..screen_width {
                let upper_color = frame_buffer[upper_y][x];
                let lower_color = if lower_y < double_height {
                    frame_buffer[lower_y][x]
                } else {
                    frame_buffer[upper_y][x] // Fallback if out of bounds
                };
                
                // Set foreground (upper half) and background (lower half) colors
                if upper_color != current_fg || lower_color != current_bg {
                    output.push_str(&format!("\x1b[38;5;{}m\x1b[48;5;{}m", upper_color, lower_color));
                    current_fg = upper_color;
                    current_bg = lower_color;
                }
                
                // Use upper half block character (▀) - shows upper color as foreground, lower as background
                output.push('▀');
            }
            
            // Reset color at end of line and move to next
            if y < screen_height - 1 {
                output.push_str("\x1b[0m\r\n");
                current_fg = 0;
                current_bg = 0;
            }
        }
        
        // Reset color and write everything at once
        output.push_str("\x1b[0m");
        write!(stdout, "{}", output)?;
        stdout.flush()?;
        
        Ok(())
    }
    
    // Convert distance to 256-color code for walls
    // Uses warm color gradient for better visual appeal
    fn distance_to_color(&self, distance: f64) -> u8 {
        // Clamp distance to reasonable range (0.1 to 15.0)
        let clamped_dist = distance.max(0.1).min(15.0);
        
        // Use logarithmic scale for better depth perception
        let log_dist = (clamped_dist + 1.0f64).ln();
        let max_log = (15.0f64 + 1.0f64).ln();
        let normalized = 1.0 - (log_dist / max_log);
        
        // Use warm color palette: bright yellow/orange for close, dark red for far
        // Colors 220-226 are warm yellows/oranges, 88-94 are dark reds
        if normalized > 0.5 {
            // Close walls: bright warm colors (220-226)
            let warm = 220.0 + ((normalized - 0.5) * 12.0);
            warm.max(220.0).min(226.0) as u8
        } else {
            // Far walls: dark red/brown (88-94)
            let dark = 88.0 + (normalized * 12.0);
            dark.max(88.0).min(94.0) as u8
        }
    }
    
    // Ceiling color gradient - sky blue tones
    fn ceiling_color(&self, dist_from_center: f64) -> u8 {
        // Lighter blue near horizon, darker blue at top
        let normalized = dist_from_center.min(1.0);
        // Use sky blue colors: 39-45 range (bright to medium blue)
        let blue_shade = 39.0 + (normalized * 6.0);
        blue_shade.max(39.0).min(45.0) as u8
    }
    
    // Floor color gradient - dark stone/concrete
    fn floor_color(&self, dist_from_center: f64) -> u8 {
        // Darker as we go down
        let normalized = dist_from_center.min(1.0);
        // Use dark gray/stone colors: 238-244 range (dark to medium gray)
        let gray_shade = 238.0 + (normalized * 6.0);
        gray_shade.max(238.0).min(244.0) as u8
    }

    fn update(&mut self, keys: &[KeyCode]) {
        let mut move_x = 0.0;
        let mut move_y = 0.0;
        let mut rotate = 0.0;
        
        for key in keys {
            match key {
                KeyCode::Char('w') | KeyCode::Up => {
                    move_x += self.player.angle.cos() * MOVE_SPEED;
                    move_y += self.player.angle.sin() * MOVE_SPEED;
                }
                KeyCode::Char('s') | KeyCode::Down => {
                    move_x -= self.player.angle.cos() * MOVE_SPEED;
                    move_y -= self.player.angle.sin() * MOVE_SPEED;
                }
                KeyCode::Char('a') => {
                    move_x += self.player.angle.sin() * MOVE_SPEED;
                    move_y -= self.player.angle.cos() * MOVE_SPEED;
                }
                KeyCode::Char('d') => {
                    move_x -= self.player.angle.sin() * MOVE_SPEED;
                    move_y += self.player.angle.cos() * MOVE_SPEED;
                }
                KeyCode::Left => {
                    rotate -= ROTATION_SPEED;
                }
                KeyCode::Right => {
                    rotate += ROTATION_SPEED;
                }
                _ => {}
            }
        }
        
        // Collision detection
        let new_x = self.player.x + move_x;
        let new_y = self.player.y + move_y;
        
        if new_x >= 0.0
            && new_x < MAP_WIDTH as f64
            && new_y >= 0.0
            && new_y < MAP_HEIGHT as f64
        {
            let map_x = new_x.floor() as usize;
            let map_y = new_y.floor() as usize;
            
            if self.get_map_value(map_x, map_y) == 0 {
                self.player.x = new_x;
                self.player.y = new_y;
            }
        }
        
        self.player.angle += rotate;
        
        // Normalize angle
        while self.player.angle < 0.0 {
            self.player.angle += 2.0 * std::f64::consts::PI;
        }
        while self.player.angle >= 2.0 * std::f64::consts::PI {
            self.player.angle -= 2.0 * std::f64::consts::PI;
        }
    }
}

fn main() -> io::Result<()> {
    let mut stdout = stdout();
    
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Hide)?;
    
    let mut raycaster = Raycaster::new();
    let mut last_frame = Instant::now();
    let frame_duration = Duration::from_millis(16); // ~60 FPS
    
    loop {
        let mut keys_pressed = Vec::new();
        
        // Non-blocking event polling
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) = event::read()?
            {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        execute!(stdout, Show, LeaveAlternateScreen)?;
                        terminal::disable_raw_mode()?;
                        return Ok(());
                    }
                    _ => keys_pressed.push(code),
                }
            }
        }
        
        raycaster.update(&keys_pressed);
        raycaster.render(&mut stdout)?;
        
        // Frame rate limiting
        let elapsed = last_frame.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
        last_frame = Instant::now();
    }
}

