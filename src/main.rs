use std::sync::OnceLock;

use bevy::{prelude::*, window::{WindowResolution, WindowMode}};
use rand::Rng;

const UP_SPEED: f32 = 500.0;
const GRAVITY: f32 = -2000.0;
const ANGLE_AMPLITUDE: f32 = 0.8;
const PIPE_WINDOW_SIZE: f32 = 250.0;
const PIPE_START_SPEED: f32 = 100.0;
const PIPE_MAX_SPEED: f32 = 1000.0;
const PIPE_TIME_TO_MAX: f32 = 60.0;
const PIPE_GAP: f32 = 500.0;

static WINDOW_SIZE: OnceLock<WindowResolution> = OnceLock::new();

type LoadCallback = Box<dyn Send + Sync + FnOnce(Vec<HandleUntyped>, &mut Commands)>;

struct LoadingBundle {
    handles: Vec<HandleUntyped>,
    on_load: LoadCallback,
}

#[derive(Resource, Default)]
struct LoadingAssets(Vec<LoadingBundle>);

impl std::ops::Deref for LoadingAssets {
    type Target = Vec<LoadingBundle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for LoadingAssets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Component, Default)]
struct Movable {
    velocity: Vec3,
    acceleration: Vec3,
}

#[derive(Component, Default)]
struct Player;

#[derive(Component, Default)]
struct Pipe;

#[derive(Bundle, Default)]
struct PipeBundle {
    movable: Movable,
    sprite: SpriteBundle, // for computer visibility and global transform
    marker: Pipe,
}

#[derive(Bundle, Default)]
struct PlayerBundle {
    movable: Movable,
    sprite: SpriteBundle,
    marker: Player,
}

fn post_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut bundles: ResMut<LoadingAssets>,
) {
    use bevy::asset::LoadState::*;
    let mut i = 0;
    while i < bundles.len() {
        let loaded = bundles[i]
            .handles
            .iter()
            .all(|handle| matches!(asset_server.get_load_state(handle), Loaded));

        if loaded {
            let bundle = bundles.remove(i);
            (bundle.on_load)(bundle.handles, &mut commands);
        } else {
            i += 1;
        }
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(PlayerBundle {
        sprite: SpriteBundle {
            texture: asset_server.load("sprites/bird.png"),
            ..default()
        },
        movable: Movable {
            acceleration: Vec3::Y * GRAVITY,
            ..default()
        },
        ..default()
    });

    let pipe_start = asset_server.load("sprites/pipe.png");
    let pipe_segment = asset_server.load("sprites/pipe_piece.png");

    let pipe_start_height = 192.0;

    let pipe_segment_height = 96.0;

    let lower_pipe_bundle = SpriteBundle {
        texture: pipe_start,
        transform: Transform {
            translation: Vec3::NEG_Y * (pipe_start_height + PIPE_WINDOW_SIZE) / 2.0,
            ..default()
        },
        ..default()
    };

    let mut upper_pipe_bundle = lower_pipe_bundle.clone();
    upper_pipe_bundle.sprite.flip_y = true;
    upper_pipe_bundle.transform.translation *= -1.0;

    let mut rng = rand::thread_rng();

    let mut spawn_pipe = |x: f32| {
        commands
            .spawn(PipeBundle {
                movable: Movable {
                    acceleration: Vec3::NEG_X * (PIPE_MAX_SPEED - PIPE_START_SPEED)
                        / PIPE_TIME_TO_MAX,
                    velocity: Vec3::NEG_X * PIPE_START_SPEED,
                },
                sprite: SpriteBundle {
                    transform: Transform {
                        translation: Vec3 {
                            x,
                            y: rng.gen(),
                            ..default()
                        },
                        ..default()
                    },
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                parent
                    .spawn(lower_pipe_bundle.clone())
                    .with_children(|parent| {
                        for i in 0..10 {
                            parent.spawn(SpriteBundle {
                                texture: pipe_segment.clone(),
                                transform: Transform {
                                    translation: Vec3::NEG_Y
                                        * pipe_segment_height
                                        * (1 + 2 * i) as f32
                                        / 2.0,
                                    ..default()
                                },
                                ..default()
                            });
                        }
                    });
                parent
                    .spawn(upper_pipe_bundle.clone())
                    .with_children(|parent| {
                        for i in 0..10 {
                            parent.spawn(SpriteBundle {
                                texture: pipe_segment.clone(),
                                transform: Transform {
                                    translation: Vec3::Y * pipe_segment_height * (1 + 2 * i) as f32
                                        / 2.0,
                                    ..default()
                                },
                                ..default()
                            });
                        }
                    });
            });
    };
    let right_border = WINDOW_SIZE.get().unwrap().width() / 2.0 + 100.0;
    for i in 0..10 {
        spawn_pipe(right_border + i as f32 * PIPE_GAP);
    }
}

fn jump(keyboard_input: Res<Input<KeyCode>>, mut query: Query<&mut Movable, With<Player>>) {
    if keyboard_input.pressed(KeyCode::Space) {
        let mut movable = query.single_mut();
        movable.velocity = Vec3::Y * UP_SPEED;
    }
}

fn rotate(mut query: Query<(&mut Transform, &Movable), With<Player>>) {
    for (mut transform, movable) in &mut query {
        use std::f32::consts::FRAC_PI_2;
        let angle =
            ((movable.velocity.y / UP_SPEED) * ANGLE_AMPLITUDE).clamp(-FRAC_PI_2, FRAC_PI_2);
        transform.rotation = Quat::from_axis_angle(Vec3::Z, angle);
    }
}

fn apply_acceleration(time: Res<Time>, mut query: Query<&mut Movable>) {
    let dt = time.delta_seconds();
    for mut movable in &mut query {
        movable.velocity = movable.velocity + movable.acceleration * dt;
    }
}

fn apply_velocity(time: Res<Time>, mut query: Query<(&Movable, &mut Transform)>) {
    let dt = time.delta_seconds();
    for (movable, mut transform) in &mut query {
        transform.translation += movable.velocity * dt;
    }
}

fn reuse_pipes(mut query: Query<&mut Transform, With<Pipe>>) {
    let left_border = -WINDOW_SIZE.get().unwrap().width() / 2.0 - 100.0;
    let mut farther_position = query
        .iter()
        .map(|x| x.translation)
        .max_by(|t1, t2| t1.x.partial_cmp(&t2.x).unwrap())
        .unwrap();
    for mut transform in &mut query {
        if transform.translation.x < left_border {
            transform.translation = farther_position;
            transform.translation.x += PIPE_GAP;
            farther_position = transform.translation;
        }
    }
}

fn main() {
    WINDOW_SIZE
        .set(WindowResolution::new(1280.0, 720.0))
        .expect("Could not initialize window resolution");
    println!(
        "Width: {}, Height: {}",
        WINDOW_SIZE.get().unwrap().width(),
        WINDOW_SIZE.get().unwrap().height(),
    );

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WINDOW_SIZE.get().unwrap().clone(),
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .add_startup_system(startup)
        .init_resource::<LoadingAssets>()
        .add_system(post_loading)
        .add_systems((
            jump,
            apply_acceleration.after(jump),
            apply_velocity.after(apply_acceleration),
            rotate.after(apply_acceleration),
            reuse_pipes,
        ))
        .run();
}
