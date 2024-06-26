//!
//! Toy Entity Component System using various methods to schedule systems
//! 

use std::{io::Stdout, sync::{Arc, RwLock}, time::{SystemTime, UNIX_EPOCH}};
use std::io::{stdout, Result};
use std::time::Duration;

use clap::Parser;

use threadpool::ThreadPool;

use rand::random;

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen
    },
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{canvas::Canvas, Block, Borders},
};

const WIDTH: isize = 212;
const MIN_X: isize = -106;
const MAX_X: isize = 105;

const HEIGHT: isize = 50;
const MIN_Y: isize = -25;
const MAX_Y: isize = 24;

const MAX_HEALTH: usize = 10;

/// Status of the particles
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Dead,
    Low,
    Medium,
    High,
}

/// Update the positions based on velocities
fn position_update_system(
    world: Arc<World>,
) {
    let mut positions = world.positions.write().unwrap();
    let velocities = world.velocities.read().unwrap();
    for (position, velocity) in positions.iter_mut().zip(velocities.iter()).filter(|v| v.0.is_some() && v.1.is_some()) {
        *position = Some((
            (position.unwrap().0 + velocity.unwrap().0).clamp(MIN_X, MAX_X),
            (position.unwrap().1 + velocity.unwrap().1).clamp(MIN_Y, MAX_Y),
        ));
    }
}

/// Update velocities based on acceleration
fn velocity_update_system(
    world: Arc<World>,
) {
    let mut velocities = world.velocities.write().unwrap();
    let accelerations = world.accelerations.read().unwrap();
    for (velocity, acceleration) in velocities.iter_mut().zip(accelerations.iter()).filter(|v| v.0.is_some() && v.1.is_some()) {
        *velocity = Some((
            velocity.unwrap().0 + acceleration.unwrap().0,
            velocity.unwrap().1 + acceleration.unwrap().1,
        ));
    }
}

/// Render positions onto a square canvas
fn component_render_system(
    world: Arc<World>,
) {
    let positions = world.positions.read().unwrap();
    let mut new_canvas = [[Status::Dead; WIDTH as usize]; HEIGHT as usize];
    let healths = world.healths.read().unwrap();
    let alives = world.alives.read().unwrap();
    for ((position, _alive), health) in positions.iter().zip(alives.iter()).zip(healths.iter()).filter(|v| v.0.0.is_some() && v.0.1.is_some() && v.0.1.unwrap() && v.1.is_some()) {
        let x = (position.unwrap().0.clamp(MIN_X, MAX_X) + WIDTH / 2) as usize;
        let y = (position.unwrap().1.clamp(MIN_Y, MAX_Y) + HEIGHT / 2) as usize;

        match health.unwrap() {
            7.. => new_canvas[y][x] = Status::High,
            4..=6 => new_canvas[y][x] = Status::Medium,
            1..=3 => new_canvas[y][x] = Status::Low,
            _ => (),
        }
    }
    (*world.canvas.write().unwrap()) = new_canvas;
}

/// Update Entity Healths
fn health_update_system(
    world: Arc<World>,
) {
    let mut healths = world.healths.write().unwrap();
    let health_changes = world.health_changes.read().unwrap();
    for (health, change) in healths.iter_mut().zip(health_changes.iter()).filter(|v| v.0.is_some() && v.1.is_some()) {
        *health = Some(health.unwrap().saturating_add_signed(change.unwrap()).clamp(0, MAX_HEALTH));
    }
}

/// Print the canvas to the screen
fn canvas_render_system(
    world: Arc<World>,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let then = *world.last_render.read().unwrap();
    *world.last_render.write().unwrap() = now;
    let frame_rate = if now - then == 0 {
        60
    } else {
        1000 / (now - then)
    };
    terminal.draw(|frame| {
        let area = frame.size();
        frame.render_widget(
            Canvas::default()
                .block(
                    Block::default()
                    .borders(Borders::ALL)
                    .title(
                        format!(
                            "Living Entities: {} ----- ({} fps)",
                            (*world.living_entities.read().unwrap()),
                            frame_rate,
                        )
                    )
                )
                .background_color(Color::Black)
                .x_bounds([0.0, WIDTH as f64])
                .y_bounds([0.0, HEIGHT as f64])
                .paint(|ctx| {
                let canvas = world.canvas.read().unwrap();
                for (y, row) in canvas.iter().enumerate() {
                    for (x, item) in row.iter().enumerate() {
                        match *item {
                            Status::Dead => ctx.print(x as f64, y as f64, "_".gray()),
                            Status::Low => ctx.print(x as f64, y as f64, "X".red()),
                            Status::Medium => ctx.print(x as f64, y as f64, "X".yellow()),
                            Status::High => ctx.print(x as f64, y as f64, "X".green()),
                        }
                    }
                }
            }),
            area,
        );
    })?;
    Ok(())
}

/// Update the accelerations randomly
fn acceleration_update_system(
    world: Arc<World>,
) {
    let mut accelerations = world.accelerations.write().unwrap();
    for acceleration in accelerations.iter_mut() {
        match acceleration {
            Some(_) => *acceleration = None,
            None => {
                let left_right = match random::<usize>() % 4 {
                    0 => 1,
                    1 => -1,
                    _ => 0,
                };

                let up_down = match random::<usize>() % 4 {
                    0 => 1,
                    1 => -1,
                    _ => 0,
                };

                *acceleration = Some((left_right, up_down));
            },
        }
    }
}

/// Check if Healths are 0 (if so mark the entity as dead)
fn alive_system(
    world: Arc<World>,
) {
    let mut alives = world.alives.write().unwrap();
    let healths = world.healths.read().unwrap();
    for (alive, health) in alives.iter_mut().zip(healths.iter()).filter(|v| v.0.is_some()) {
        if health.is_none() || health.unwrap() == 0 {
            *alive = Some(false);
        }
    }
}

/// Add damage or healing to entities
fn health_changes_system(
    world: Arc<World>,
) {
    let mut health_changes = world.health_changes.write().unwrap();
    for health_change in health_changes.iter_mut() {
        match health_change {
            Some(_) => *health_change = None,
            None => {
                if random() {
                    *health_change = Some(random::<isize>() % 2);
                } else {
                    *health_change = Some(-(random::<isize>() % 2));
                }
            }
        }
    }
}

/// Prints the Number of Alive Entities
fn alive_entities_display_system(
    world: Arc<World>,
) {
    let alive = world.alives.read().unwrap();
    let total_alive = alive.iter().filter(|v| v.is_some() && v.unwrap()).count();
    (*world.living_entities.write().unwrap()) = total_alive;
}

/// User Input System
fn user_input_system(
    world: Arc<World>,
) {
    if event::poll(Duration::from_millis(5)).unwrap() {
        if let event::Event::Key(key) = event::read().unwrap() {
            if key.kind == KeyEventKind::Press &&
                key.code == KeyCode::Char('q') {
                (*world.running.write().unwrap()) = false;
            }
        }
    }
}

/// Entity Component System
pub struct World {
    // Whether the simulation is running
    running: RwLock<bool>,
    // Last Screen Render Time
    last_render: RwLock<u128>,

    // The Entities in the World
    _entities: Vec<usize>,
    // Number of living entities
    living_entities: Arc<RwLock<usize>>,

    // Positions for entities
    positions: Arc<RwLock<Vec<Option<(isize, isize)>>>>,
    /// Velocities for moving entities
    velocities: Arc<RwLock<Vec<Option<(isize, isize)>>>>,
    /// Accelerations for moving entities
    accelerations: Arc<RwLock<Vec<Option<(isize, isize)>>>>,
    /// Whether or not entities are alive
    alives: Arc<RwLock<Vec<Option<bool>>>>,
    /// Canvas to display
    canvas: Arc<RwLock<[[Status; WIDTH as usize]; HEIGHT as usize]>>,
    /// Health of Components
    healths: Arc<RwLock<Vec<Option<usize>>>>,
    /// Health Changes
    health_changes: Arc<RwLock<Vec<Option<isize>>>>,
}

impl World {
    pub fn new(entities: usize) -> Self {
        let mut entity_list = Vec::with_capacity(entities);
        let mut positions = Vec::with_capacity(entities);
        let mut velocities = Vec::with_capacity(entities);
        let mut accelerations = Vec::with_capacity(entities);
        let mut alives = Vec::with_capacity(entities);
        let mut healths = Vec::with_capacity(entities);
        let mut health_changes = Vec::with_capacity(entities);

        for id in 0..entities {
            entity_list.push(id);
            positions.push(if random() {
                Some((random::<isize>().clamp(MIN_X, MAX_X), random::<isize>().clamp(MIN_Y, MAX_Y)))
            } else {
                None
            });
            positions.push(random::<Option<(isize, isize)>>());
            velocities.push(if random() { Some((0, 0)) } else { None });
            accelerations.push(if random() { Some((0, 0)) } else { None });
            alives.push(Some(true));
            healths.push(Some(random::<usize>() % 100));
            health_changes.push(None);
        }

        Self {
            running: RwLock::new(true),
            last_render: RwLock::new(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
            _entities: entity_list,
            living_entities: Arc::new(RwLock::new(0)),
            positions: Arc::new(RwLock::new(positions)),
            velocities: Arc::new(RwLock::new(velocities)),
            accelerations: Arc::new(RwLock::new(accelerations)),
            alives: Arc::new(RwLock::new(alives)),
            canvas: Arc::new(RwLock::new([[Status::Dead; WIDTH as usize]; HEIGHT as usize])),
            healths: Arc::new(RwLock::new(healths)),
            health_changes: Arc::new(RwLock::new(health_changes)),
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Number of Entities to Spawn
    #[arg(short, long, default_value_t = 100_000)]
    entities: usize,

    // Workers in the threadpool
    #[arg(short, long, default_value_t = 3)]
    workers: usize,

    // Whether to log the alive entities at the end
    #[arg(short, long, default_value_t = false)]
    log_living: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let world = Arc::new(World::new(args.entities));
    let pool = ThreadPool::new(args.workers);

    let systems = vec![
        user_input_system,
        position_update_system,
        velocity_update_system,
        component_render_system,
        health_update_system,
        acceleration_update_system,
        alive_system,
        health_changes_system,
        alive_entities_display_system,
    ];
    let systems_len = systems.len();

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    while *world.running.read().unwrap() {
        // Draw UI
        canvas_render_system(world.clone(), &mut terminal)?;

        // Handle Events
        for i in 0..systems_len {
            let c_world = world.clone();
            let f = systems[i];
            pool.execute(move || f(c_world));
        }

        pool.join();
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    if args.log_living {
        for (i, _) in world.alives.read().unwrap().iter().filter(|v| v.is_some() && v.unwrap()).enumerate() {
            println!(
                "Entity {{ id: {}, position: {:?}, velocity: {:?}, acceleration: {:?}, alive: true, health: {:?}, health_changes: {:?} }}",
                i,
                world.positions.read().unwrap()[i],
                world.velocities.read().unwrap()[i],
                world.accelerations.read().unwrap()[i],
                world.healths.read().unwrap()[i],
                world.health_changes.read().unwrap()[i],
            );
        }
    }

    Ok(())
}
