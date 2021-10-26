use bevy::input::mouse::*;
use bevy::pbr::AmbientLight;
use bevy::prelude::*;
use bevy_mod_picking::*;
use rand::*;
use wasm_bindgen::prelude::*;

mod game;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum GameState {
    Menu,
    Playing,
    Over,
    Restart,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
}

macro_rules! game {
    ($self: ident, $method: ident, $x:expr, $y:expr) => {{
        $self
            .$method($x, $y)
            .expect(&format!("Failed to access cell at {}, {}", $x, $y))
    }};
}

/// Anchor structs
///
/// Describes the game tile. Holds game coordinates.
#[derive(Debug, Default)]
struct Tile {
    x: u8,
    y: u8,
}
/// Minesweeper Bevy plugin
struct Minesweeper;
/// Displays the number of adjacent mines.
///
/// Pops up when user hovers over a tile
struct TileMines;
/// Displays the 'expected' number of mines left to uncover
///
/// Note: this label only displays the real number of mines
/// in the beginning. After that - every time a user flags
/// a tile - the value gets decreased, regardless of the mine
/// being present or no.
struct MinesLeft;
/// Restarts the game with the current configuration
struct RestartButton;

/// Displays the GameOver™ notification
struct GameOver;
/// Holds the game UI: [MinesLeft], [RestartButton] and [GameTimer]
struct GameUI;
/// Ambient light
struct GameLight;
/// Brings the user back to the Menu
struct BackButton;
/// Perspective camera
struct UICamera;

/// Holds the game menu: difficulty selection
struct MenuUI;
/// Displays the elapsed time
struct GameTimer {
    timer: Timer,
    ticks: u64,
}

/// Used for orbiting the camera around the board (only around Y-axis)
///
/// I took this code from https://bevy-cheatbook.github.io/cookbook/pan-orbit-camera.html
/// and adjusted it to not pan and to not pitch
struct OrbitCamera {
    /// The "focus point" to orbit around.
    focus: Vec3,
    radius: f32,
    upside_down: bool,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        OrbitCamera {
            focus: Vec3::ZERO,
            radius: 5.0,
            upside_down: false,
        }
    }
}

/// Holds meshes, fonts, scenes and other materials used by this game
struct GameMaterials {
    text_font: Handle<Font>,
    digit_font: Handle<Font>,
    notification_font: Handle<Font>,
    tile_normal: Handle<StandardMaterial>,
    tile_hovered: Handle<StandardMaterial>,
    tile: Handle<Mesh>,
    mine: Handle<Scene>,
    flag: Handle<Scene>,
    smiley: Handle<ColorMaterial>,
    transparent: Handle<ColorMaterial>,
    empty: Handle<Scene>,
    trees: Handle<Scene>,
}

impl FromWorld for GameMaterials {
    fn from_world(world: &mut World) -> Self {
        let (
            text_font,
            digit_font,
            notification_font,
            tile_normal,
            tile_hovered,
            tile,
            smile,
            mine,
            flag,
            empty,
            trees,
        ) = world
            .get_resource::<AssetServer>()
            .map(|asset_server| {
                (
                    asset_server.load("fonts/FiraSans-Bold.ttf"),
                    asset_server.load("fonts/QuirkyRobot.ttf"),
                    asset_server.load("fonts/data-latin.ttf"),
                    asset_server.load("models/tile.glb#Material1"),
                    asset_server.load("models/tile.glb#Material0"),
                    asset_server.load("models/tile.glb#Mesh0/Primitive1"),
                    asset_server.load("icons/smile.png"),
                    asset_server.load("models/target.glb#Scene0"),
                    asset_server.load("models/flag.glb#Scene0"),
                    asset_server.load("models/tile.glb#Scene0"),
                    asset_server.load("models/tile_treeQuad.glb#Scene0"),
                )
            })
            .expect("Couldn't get world asset server");

        let (smiley, transparent) = world
            .get_resource_mut::<Assets<ColorMaterial>>()
            .map(|mut materials| {
                (
                    materials.add(smile.into()),
                    materials.add(ColorMaterial {
                        color: Color::NONE,
                        ..Default::default()
                    }),
                )
            })
            .expect("Couldn't get color materials");

        GameMaterials {
            tile_normal,
            tile_hovered,
            flag,
            text_font,
            digit_font,
            notification_font,
            tile,
            mine,
            smiley,
            transparent,
            empty,
            trees,
        }
    }
}

impl Plugin for Minesweeper {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .init_resource::<GameMaterials>()
        .add_state(GameState::Menu)
        .add_system_set(
            SystemSet::on_enter(GameState::Menu)
                .with_system(cleanup_board.system())
                .with_system(cleanup_ui.system())
                .with_system(cleanup_camera.system())
                .with_system(setup_menu.system()),
        )
        .add_system_set(SystemSet::on_update(GameState::Menu).with_system(handle_menu.system()))
        .add_system_set(SystemSet::on_exit(GameState::Menu).with_system(cleanup_menu.system()))
        .add_system_set(
            SystemSet::on_enter(GameState::Playing)
                .with_system(setup_scene.system())
                .with_system(setup_board.system())
                .with_system(setup_ui.system()),
        )
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(handle_mouse_action.system())
                .with_system(handle_highlight.system())
                .with_system(update_mines.system())
                .with_system(update_timer.system())
                .with_system(orbit_camera.system())
                .with_system(handle_restart.system())
                .with_system(handle_back.system()),
        )
        .add_system_set(SystemSet::on_enter(GameState::Over).with_system(game_over.system()))
        .add_system_set(
            SystemSet::on_update(GameState::Over)
                .with_system(handle_restart.system())
                .with_system(handle_back.system()),
        )
        .add_system_set(
            SystemSet::on_enter(GameState::Restart)
                .with_system(cleanup_board.system())
                .with_system(cleanup_ui.system())
                .with_system(cleanup_camera.system())
                .with_system(restart.system()),
        );
    }
}

/// Sets up the game menu which allows for difficulty level selection
fn setup_menu(mut commands: Commands, materials: Res<GameMaterials>) {
    commands
        .spawn_bundle(UiCameraBundle::default())
        .insert(UICamera);

    let text_style = TextStyle {
        font: materials.notification_font.clone(),
        font_size: 60.0,
        color: Color::WHITE,
    };

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.), Val::Percent(20.)),
                margin: Rect::all(Val::Auto),
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: materials.transparent.clone(),
            ..Default::default()
        })
        .insert(MenuUI)
        .with_children(|parent| {
            parent
                .spawn_bundle(ButtonBundle {
                    material: materials.transparent.clone(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text::with_section("Easy", text_style.clone(), Default::default()),
                        ..Default::default()
                    });
                })
                .insert(DifficultyLevel::Easy);
            parent
                .spawn_bundle(ButtonBundle {
                    material: materials.transparent.clone(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text::with_section("Medium", text_style.clone(), Default::default()),
                        ..Default::default()
                    });
                })
                .insert(DifficultyLevel::Medium);

            parent
                .spawn_bundle(ButtonBundle {
                    material: materials.transparent.clone(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text::with_section("Hard", text_style.clone(), Default::default()),
                        ..Default::default()
                    });
                })
                .insert(DifficultyLevel::Hard);
        });
}

/// Handles user interactions with the menu
///
/// Starts the new game (changes to [GameState::Playing]) when a user selects
/// a difficulty level.
/// Handles hovering logic: the label text will increas by 20% if hovered
/// over.
fn handle_menu(
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
    mut interaction_query: Query<
        (&Interaction, &DifficultyLevel, &Children),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, level, children) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                if let Ok(mut text) = text_query.get_mut(children[0]) {
                    text.sections[0].style.font_size *= 1.2;
                }
            }
            Interaction::Clicked => {
                let game = match level {
                    DifficultyLevel::Easy => game::Game::new(5, 5),
                    DifficultyLevel::Medium => game::Game::new(10, 10),
                    DifficultyLevel::Hard => game::Game::new(15, 15),
                };

                info!("\n{}", game);

                commands.remove_resource::<game::Game>();
                commands.insert_resource(game);

                state
                    .set(GameState::Playing)
                    .expect("Failed to change the state");
            }
            Interaction::None => {
                if let Ok(mut text) = text_query.get_mut(children[0]) {
                    text.sections[0].style.font_size /= 1.2;
                }
            }
        }
    }
}

/// Cleans up the game menu
fn cleanup_menu(mut commands: Commands, querry: Query<Entity, Or<(With<MenuUI>, With<UICamera>)>>) {
    for entity in querry.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Sets up a 3D scene
///
/// Settings up the scene in this case includes setting up a
/// perspective camera and light.
fn setup_scene(mut commands: Commands) {
    let translation = Vec3::new(0., 15., 15.0);
    let target = Vec3::ZERO;
    let radius = translation.length();

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_translation(translation).looking_at(target, Vec3::Y),
            ..Default::default()
        })
        .insert(OrbitCamera {
            radius,
            focus: target,
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default());
    commands
        .spawn_bundle(LightBundle {
            transform: Transform::from_xyz(3.0, 5.0, 3.0),
            ..Default::default()
        })
        .insert(GameLight);
}

/// Cleans up the 3D scene by despawning its components
fn cleanup_camera(
    mut commands: Commands,
    query: Query<Entity, Or<(With<OrbitCamera>, With<GameLight>)>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Sets up the [GameUI] components: mines, timer labels and restart, back buttons
///
/// Components are configured to be a part of one bundle and their coordinates
/// are relative to that bundle.
fn setup_ui(
    mut commands: Commands,
    window: Res<WindowDescriptor>,
    materials: Res<GameMaterials>,
    game: Res<game::Game>,
) {
    commands
        .spawn_bundle(UiCameraBundle::default())
        .insert(UICamera);

    let text_style = TextStyle {
        font: materials.digit_font.clone(),
        font_size: 40.,
        color: Color::WHITE,
    };

    let h = window.height / 10.;
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Px(h)),
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(0.),
                    left: Val::Px(0.),
                    ..Default::default()
                },
                // TODO: figure out how to center items properly
                border: Rect {
                    top: Val::Px(5.),
                    left: Val::Px(15.),
                    right: Val::Px(25.),
                    ..Default::default()
                },
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: materials.transparent.clone(),
            ..Default::default()
        })
        .insert(GameUI)
        .with_children(|parent| {
            // Spawn timer label
            parent
                .spawn_bundle(TextBundle {
                    text: Text::with_section("Time:", text_style.clone(), Default::default()),
                    style: Style {
                        size: Size::new(Val::Percent(10.), Val::Percent(100.)),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(GameTimer {
                    timer: Timer::from_seconds(1., true),
                    ticks: 0,
                });

            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(h), Val::Px(h)),
                        ..Default::default()
                    },
                    material: materials.smiley.clone(),
                    ..Default::default()
                })
                .insert(RestartButton);

            // Spawn mines label
            parent
                .spawn_bundle(TextBundle {
                    text: Text::with_section(
                        format!("Left: {}", game.mines()),
                        text_style.clone(),
                        Default::default(),
                    ),
                    style: Style {
                        size: Size::new(Val::Percent(10.), Val::Percent(100.)),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(MinesLeft);
        });

    // Spawn 'Back' button
    commands
        .spawn_bundle(ButtonBundle {
            material: materials.transparent.clone(),
            style: Style {
                size: Size::new(Val::Px(h), Val::Px(h)),
                position_type: PositionType::Absolute,
                position: Rect {
                    left: Val::Px(0.),
                    bottom: Val::Px(0.),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text::with_section("< Back", text_style, Default::default()),
                ..Default::default()
            });
        })
        .insert(BackButton);
}

/// Cleans up the [GameUI] components by despawning them
fn cleanup_ui(
    mut commands: Commands,
    ui_query: Query<
        Entity,
        Or<(
            With<GameUI>,
            With<GameOver>,
            With<GameTimer>,
            With<BackButton>,
            With<UICamera>,
        )>,
    >,
) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Creates a graphical representation of the [game::Game]
fn setup_board(mut commands: Commands, materials: Res<GameMaterials>, game: Res<game::Game>) {
    for y in 0..game.height() {
        for x in 0..game.width() {
            let height = rand::thread_rng().gen_range(-0.1..0.1);
            // In order to place the scene at some 3D location it should
            // be spawned as child of some other bundle. In this case I'm using
            // a PbrBundle with the same tile mesh and material as the scene
            // so I can select/highligh the tile, without decompositing the scene.
            // See https://github.com/aevyrie/bevy_mod_picking/blob/master/examples/
            commands
                .spawn_bundle(PbrBundle {
                    transform: Transform::from_translation(Vec3::new(
                        x as f32 - game.width() as f32 / 2.,
                        height - 0.2,
                        y as f32 - game.height() as f32 / 2.,
                    )),
                    material: materials.tile_normal.clone(),
                    mesh: materials.tile.clone(),
                    ..Default::default()
                })
                .insert(Tile { x, y })
                .insert_bundle(PickableBundle::default())
                .with_children(|parent| {
                    parent.spawn_scene(materials.trees.clone());
                });
        }
    }
}

/// Destroys the graphical representation of the board
fn cleanup_board(mut commands: Commands, tile_query: Query<Entity, With<Tile>>) {
    for entity in tile_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Handles mouse actions.
///
/// Despite of the title, this is the main system for our game.
/// It picks the tile, user clicked on last and either opens or
/// flags it depending on the mouse button clicked.
/// Those actions may result in the state transition from
/// [GameState::Playing] to [GameState::Over] if the game was
/// won or lost.
fn handle_mouse_action(
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
    button: Res<Input<MouseButton>>,
    materials: Res<GameMaterials>,
    mut game: ResMut<game::Game>,
    picking_camera_query: Query<&PickingCamera>,
    mut tile_query: Query<(&Tile, Entity, &Children), With<Tile>>,
) {
    // First get a tile a user hovered over.
    // See https://github.com/aevyrie/bevy_mod_picking
    let (tile, entity, children) = if let Some(query) = picking_camera_query
        .iter()
        .last()
        .and_then(|picking_camera| picking_camera.intersect_top())
        .and_then(|(entity, _intersection)| tile_query.get_mut(entity).ok())
    {
        query
    } else {
        return;
    };

    trace!("{}", game!(game, dump, tile.x, tile.y));

    // If a user clicked on the cell - either open or flag it
    if button.just_pressed(MouseButton::Left) {
        game.open(tile.x, tile.y);
    } else if button.just_pressed(MouseButton::Right) {
        match game.flag(tile.x, tile.y) {
            Some(true) => {
                for entity in children.iter() {
                    commands.entity(*entity).despawn_recursive();
                }

                commands.entity(entity).with_children(|parent| {
                    parent.spawn_scene(materials.empty.clone());
                    parent.spawn_scene(materials.flag.clone());
                });
            }
            Some(false) => {
                for entity in children.iter() {
                    commands.entity(*entity).despawn_recursive();
                }

                commands.entity(entity).with_children(|parent| {
                    parent.spawn_scene(materials.trees.clone());
                });
            }
            _ => unreachable!(),
        };
    }

    // Reflect on the game state:
    // 1. If the game continues it's possible that a user clicked open and more cells were uncovered.
    // 2. If the game is won - flagged cells should be marked as mined
    // 3. If the game is lost - all mined cells should be uncovered.
    let (entities, scene) = match game.state() {
        game::GameState::Continue => {
            let entities = tile_query
                .iter()
                .filter(|(tile, _entity, _children)| {
                    game!(game, cell_state, tile.x, tile.y) == game::CellState::Uncovered
                })
                .map(|(_tile, entity, children)| (entity, children))
                .collect::<Vec<(Entity, &Children)>>();

            (entities, materials.empty.clone())
        }
        game::GameState::Won => {
            let entities = tile_query
                .iter()
                .filter(|(tile, _entity, _children)| {
                    game!(game, cell_state, tile.x, tile.y) == game::CellState::Flagged
                })
                .map(|(_tile, entity, children)| (entity, children))
                .collect::<Vec<(Entity, &Children)>>();

            state
                .set(GameState::Over)
                .expect("Failed to change the game state");
            (entities, materials.flag.clone())
        }
        game::GameState::Lost => {
            let entities = tile_query
                .iter()
                .filter(|(tile, _entity, _children)| game!(game, has_mine, tile.x, tile.y))
                .map(|(_tile, entity, children)| (entity, children))
                .collect::<Vec<(Entity, &Children)>>();

            state
                .set(GameState::Over)
                .expect("Failed to change the game state");
            (entities, materials.mine.clone())
        }
    };

    for (entity, children) in entities {
        for entity in children.iter() {
            commands.entity(*entity).despawn_recursive();
        }
        commands.entity(entity).with_children(|parent| {
            parent.spawn_scene(scene.clone());
        });
    }
}

/// Handle tile highlighting.
fn handle_highlight(
    mut commands: Commands,
    windows: Res<Windows>,
    materials: Res<GameMaterials>,
    game: Res<game::Game>,
    mut interaction_query: Query<
        (&Tile, &Interaction, &mut Handle<StandardMaterial>),
        (Or<(Changed<Interaction>, Changed<Selection>)>, With<Tile>),
    >,
    text_query: Query<Entity, With<TileMines>>,
) {
    let window = windows
        .get_primary()
        .expect("Couldn't get the primary window");

    for (tile, interaction, mut material) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                // When hovered over - show the number of adjacent mines
                let cursor_position = window.cursor_position().unwrap_or(Vec2::ZERO);
                let mines = game!(game, adjacent_mines, tile.x, tile.y);
                let state = game!(game, cell_state, tile.x, tile.y);

                if state == game::CellState::Uncovered {
                    commands
                        .spawn_bundle(TextBundle {
                            text: Text::with_section(
                                format!("Mines: {}", mines),
                                TextStyle {
                                    font: materials.text_font.clone(),
                                    font_size: 10.0,
                                    color: Color::WHITE,
                                },
                                Default::default(),
                            ),
                            style: Style {
                                position_type: PositionType::Absolute,
                                position: Rect {
                                    bottom: Val::Px(cursor_position.y + 5.),
                                    left: Val::Px(cursor_position.x),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(TileMines);
                }
                // Change the tile material
                *material = materials.tile_hovered.clone();
            }
            _ => {
                // Stop showing the number of adjacent mines
                for entity in text_query.iter() {
                    commands.entity(entity).despawn_recursive();
                }
                // Restore the tile material to its previous state
                match game!(game, cell_state, tile.x, tile.y) {
                    game::CellState::Covered => *material = materials.tile_normal.clone(),
                    game::CellState::Uncovered => *material = materials.tile_normal.clone(),
                    _ => {}
                };
            }
        }
    }
}

/// Updates the timer [GameTimer] label.
fn update_timer(
    time: Res<Time>,
    game: Res<game::Game>,
    mut text_query: Query<(&mut Text, &mut GameTimer), With<GameTimer>>,
) {
    if let Some((mut text, mut game_timer)) = text_query.iter_mut().last() {
        if game.state() != game::GameState::Continue {
            game_timer.timer.reset();
        }

        if game_timer.timer.tick(time.delta()).just_finished() {
            text.sections[0].value = format!("Time: {}s", game_timer.ticks);
            game_timer.ticks += 1;
        }
    }
}

/// Updates the [TileMines] label.
fn update_mines(game: Res<game::Game>, mut text_query: Query<&mut Text, With<MinesLeft>>) {
    if let Some(mut text) = text_query.iter_mut().last() {
        text.sections[0].value = format!("Left: {}", game.mines() - game.flagged());
    }
}

/// Checks if the [RestartButton] was pressed and schedules a restart
fn handle_restart(
    mut state: ResMut<State<GameState>>,
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<RestartButton>)>,
) {
    for interaction in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                state
                    .set(GameState::Restart)
                    .expect("Failed to reset the game state");

                break;
            }
            _ => {}
        }
    }
}

/// Checks if the [BackButton] was pressed and brings the user to
/// the difficulty selection menu.
fn handle_back(
    mut state: ResMut<State<GameState>>,
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
) {
    for interaction in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                state
                    .set(GameState::Menu)
                    .expect("Failed to reset the game state");
                break;
            }
            _ => {}
        }
    }
}

/// Restarts the game
fn restart(mut commands: Commands, mut state: ResMut<State<GameState>>, game: Res<game::Game>) {
    // TODO: make this conditional
    trace!("Restarting the game");

    commands.remove_resource::<game::Game>();
    commands.insert_resource(game::Game::new(game.height(), game.width()));

    state
        .set(GameState::Playing)
        .expect("Failed to reset the game state");
}

/// Displays the score when the game is over.
fn game_over(mut commands: Commands, game: Res<game::Game>, game_materials: Res<GameMaterials>) {
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                margin: Rect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: game_materials.transparent.clone(),
            ..Default::default()
        })
        .insert(GameOver)
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text::with_section(
                    format!(
                        "Game over. {}",
                        match game.state() {
                            game::GameState::Won => "You won!",
                            game::GameState::Lost => "You lost!",
                            _ => unreachable!(),
                        }
                    ),
                    TextStyle {
                        font: game_materials.notification_font.clone(),
                        font_size: 80.0,
                        color: Color::BLACK,
                    },
                    Default::default(),
                ),
                ..Default::default()
            });
        });
}

/// Orbits camera (only 'yaw').
fn orbit_camera(
    window: Res<WindowDescriptor>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    mut query: Query<(&mut OrbitCamera, &mut Transform), With<OrbitCamera>>,
) {
    // change input mapping for orbit and panning here
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;

    if input_mouse.pressed(MouseButton::Left) {
        for ev in ev_motion.iter() {
            rotation_move += ev.delta;
        }
    }

    for ev in ev_scroll.iter() {
        scroll += ev.y;
        // Make scrolling less sensitive. Otherwise it's unbearable on Mac.
        // TODO: make this Mac only
        scroll *= 0.5;
    }

    for (mut orbit, mut transform) in query.iter_mut() {
        let mut any = false;
        if rotation_move.length_squared() > 0.0 {
            any = true;
            let delta_x = {
                let delta = rotation_move.x / window.width * std::f32::consts::PI * 2.0;
                if orbit.upside_down {
                    -delta
                } else {
                    delta
                }
            };
            let yaw = Quat::from_rotation_y(-delta_x);
            transform.rotation = yaw * transform.rotation; // rotate around global y axis
        } else if scroll.abs() > 0.0 {
            any = true;
            orbit.radius -= scroll * orbit.radius * 0.2;
            // dont allow zoom to reach zero or you get stuck
            orbit.radius = f32::max(orbit.radius, 0.05);
        }

        if any {
            // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
            // parent = x and y rotation
            // child = z-offset
            let rot_matrix = Mat3::from_quat(transform.rotation);
            transform.translation =
                orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, orbit.radius));
        }
    }
}

#[wasm_bindgen]
pub fn run() {
    let mut app = App::build();
    app.insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: "Minesweeper".to_string(),
            width: 720.,
            height: 720.,
            resizable: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(PickingPlugin)
        .add_plugin(InteractablePickingPlugin)
        .add_plugin(Minesweeper);

    // when building for Web, use WebGL2 rendering
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app.run()
}
