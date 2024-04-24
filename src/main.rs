//!
//! Toy Entity Component System using various methods to schedule systems
//! 

use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;

use threadpool::ThreadPool;

use rand::random;

const WIDTH: isize = 200;
const MIN_X: isize = -100;
const MAX_X: isize = 99;

const HEIGHT: isize = 11;
const MIN_Y: isize = -5;
const MAX_Y: isize = 5;

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
    let mut new_canvas = [[false; WIDTH as usize]; HEIGHT as usize];
    let alives = world.alives.read().unwrap();
    for (position, _alive) in positions.iter().zip(alives.iter()).filter(|v| v.0.is_some() && v.1.is_some() && v.1.unwrap()) {
        let x = (position.unwrap().0.clamp(MIN_X, MAX_X) + WIDTH / 2) as usize;
        let y = (position.unwrap().1.clamp(MIN_Y, MAX_Y) + HEIGHT / 2) as usize;
        new_canvas[y][x] = true;
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
        *health = Some(health.unwrap().saturating_add_signed(change.unwrap()));
    }
}

/// Print the canvas to the screen
fn canvas_render_system(
    world: Arc<World>,
) {
    let canvas = world.canvas.read().unwrap();
    for row in canvas.iter() {
        for item in row {
            if *item {
                print!("O")
            } else {
                print!(" ")
            }
        }
        print!("\n")
    }
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
    for (alive, health) in alives.iter_mut().zip(healths.iter()).filter(|v| v.0.is_some() && v.1.is_some()) {
        if health.unwrap() == 0 {
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
                    *health_change = Some(random::<isize>() % 5);
                } else {
                    *health_change = Some(-(random::<isize>() % 5));
                }
            }
        }
    }
}

/// Entity Component System
pub struct World {
    // The Entities in the World
    entities: Vec<usize>,

    // Positions for entities
    positions: Arc<RwLock<Vec<Option<(isize, isize)>>>>,
    /// Velocities for moving entities
    velocities: Arc<RwLock<Vec<Option<(isize, isize)>>>>,
    /// Accelerations for moving entities
    accelerations: Arc<RwLock<Vec<Option<(isize, isize)>>>>,
    /// Whether or not entities are alive
    alives: Arc<RwLock<Vec<Option<bool>>>>,
    /// Canvas to display
    canvas: Arc<RwLock<[[bool; WIDTH as usize]; HEIGHT as usize]>>,
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
            alives.push(Some(random()));
            healths.push(if random() { Some(random::<usize>() % 100) } else { None });
            health_changes.push(None);
        }

        Self {
            entities: entity_list,
            positions: Arc::new(RwLock::new(positions)),
            velocities: Arc::new(RwLock::new(velocities)),
            accelerations: Arc::new(RwLock::new(accelerations)),
            alives: Arc::new(RwLock::new(alives)),
            canvas: Arc::new(RwLock::new([[false; WIDTH as usize]; HEIGHT as usize])),
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
}

fn main() {
    let args = Args::parse();

    let world = Arc::new(World::new(args.entities));
    let pool = ThreadPool::new(args.workers);

    let systems = vec![
        position_update_system,
        velocity_update_system,
        component_render_system,
        health_update_system,
        canvas_render_system,
        acceleration_update_system,
        alive_system,
        health_changes_system,
    ];
    let systems_len = systems.len();

    println!("Running World With {} Entities", world.entities.len());

    loop {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        for i in 0..systems_len {
            let c_world = world.clone();
            let f = systems[i];
            pool.execute(move || f(c_world));
        }
    
        pool.join();
        let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        println!("Loop took {} ms", end_time - start_time);
    }
}
