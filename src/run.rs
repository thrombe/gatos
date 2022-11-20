use anyhow::Result;
use bevy::{
    prelude::{
        App, AssetServer, BuildChildren, ButtonBundle, Camera, Camera2dBundle, Changed, ClearColor,
        Color, Commands, Component, Entity, GlobalTransform, Handle, Image, ImageBundle,
        ImagePlugin, Input, MouseButton, PluginGroup, Query, Res, Resource, TextBundle, Transform,
        Vec2, Vec3, With, ResMut,
    },
    sprite::{Sprite, SpriteBundle},
    text::{Font, Text, TextStyle},
    transform::TransformBundle,
    ui::{
        AlignItems, BackgroundColor, FlexDirection, FocusPolicy, Interaction, JustifyContent, Size,
        Style, UiRect, Val,
    },
    window::{PresentMode, WindowDescriptor, WindowPlugin, Windows},
    DefaultPlugins,
};
use bevy_rapier2d::prelude::{Collider, QueryFilter, RapierContext, RapierPhysicsPlugin};
use iyes_loopless::{
    prelude::{AppLooplessStateExt, ConditionSet},
    state::NextState,
};

pub fn run(mut app: App) -> Result<()> {
    app.insert_resource(ClearColor(Color::rgb(0.25, 0.3, 0.25)))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "cheso".to_string(),
                        present_mode: PresentMode::Fifo,
                        ..WindowDescriptor::default()
                    },
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugin(RapierPhysicsPlugin::<()>::default())
        .add_system(bevy::window::close_on_esc)
        .add_loopless_state(GameState::Loading)
        .add_enter_system(GameState::Loading, spawn)
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Loading)
                .with_system(wait_for_load)
                .into(),
        )
        .add_enter_system(GameState::Playing, spawn_ui)
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Playing)
                .with_system(spawn_gate)
                .with_system(handle_unplaced)
                .with_system(unplace_gate)
                .with_system(spawn_wires)
                .with_system(finalise_wire)
                .into(),
        )
        .run();
    Ok(())
}

fn spawn_wires(
    mut c: Commands,
    mou: Res<Input<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    windows: Res<Windows>,
    mut wire: ResMut<Wire>,
) {
    let (camera, camera_transform) = q_camera.single();
    let wnd = windows.get_primary().unwrap();
    if let Some(p) = wnd.cursor_position() {
        // get the size of the window
        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (p / window_size) * 2.0 - Vec2::ONE;
        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
        // reduce it to a 2D value
        let world_pos: Vec2 = world_pos.truncate();

        let world_pos = (world_pos/5.0).round()*5.0;

        if mou.just_pressed(MouseButton::Right) {
            let id = c.spawn((
                WireNode,
                TransformBundle {
                    local: Transform {
                        translation: Vec3::new(world_pos.x, world_pos.y, 0.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )).id();
            wire.nodes.push(id);
        } else if mou.just_released(MouseButton::Right) {
            let id = c.spawn((
                WireNode,
                TransformBundle {
                    local: Transform {
                        translation: Vec3::new(world_pos.x, world_pos.y, 0.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )).id();
            wire.nodes.push(id);

            let w = std::mem::replace(&mut *wire, Wire {nodes: Default::default()});
            c.spawn((w, UnFinalised));
        }
    }
}

fn finalise_wire(mut c: Commands, q: Query<&Transform, With<WireNode>>, mut wires: Query<(&mut Wire, Entity), With<UnFinalised>>) {
    for (wire, e) in wires.iter_mut() {
        c.entity(e).remove::<UnFinalised>();
        
        wire.nodes.iter().cloned().for_each(|e| {
            c.entity(e).insert(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.4, 0.5, 0.4, 1.0),
                    custom_size: Some(Vec2::new(10., 10.)/2.0),
                    ..Default::default()
                },
                transform: Transform {
                    translation: q.get(e).unwrap().translation,
                    ..Default::default()
                },
                ..Default::default()
            });
        });
    }
}

#[derive(Component)]
pub struct WireNode;

#[derive(Component)]
pub struct UnFinalised;

#[derive(Resource, Component)]
pub struct Wire {
    pub nodes: Vec<Entity>,
}

fn wait_for_load(mut c: Commands, assets: Option<Res<Assets>>) {
    if assets.is_some() {
        c.insert_resource(NextState(GameState::Playing));
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    Loading,
    Playing,
}

fn spawn(mut c: Commands, asset_server: Res<AssetServer>) {
    c.spawn(Camera2dBundle::default());
    c.insert_resource(Wire {nodes: vec![]});
    c.insert_resource(Assets {
        and_gate: asset_server.load("sprites/and_gate.png"),
        or_gate: asset_server.load("sprites/or_gate.png"),
        not_gate: asset_server.load("sprites/not_gate.png"),
        font: asset_server.load("fonts/VarelaRound-Regular.ttf"),
    });
}

fn spawn_ui(mut c: Commands, assets: Res<Assets>) {
    c.spawn((
        ButtonBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(10.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                flex_direction: FlexDirection::Row,
                margin: UiRect {
                    left: Val::Percent(1.0),
                    right: Val::Percent(1.0),
                    top: Val::Percent(1.0),
                    bottom: Val::Percent(1.0),
                },
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        },
        GatePalette,
    ))
    .with_children(|p| {
        for g in [Gate::And, Gate::Or, Gate::Not] {
            p.spawn((
                ButtonBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        size: Size::new(Val::Auto, Val::Percent(100.0)),
                        // padding: UiRect::all(Val::Px(1.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    background_color: Color::NONE.into(),
                    focus_policy: FocusPolicy::Pass,
                    ..Default::default()
                },
                g,
            ))
            .with_children(|p| {
                p.spawn(TextBundle {
                    text: Text::from_section(
                        format!("{g:#?} Gate"),
                        TextStyle {
                            font: assets.font.clone(),
                            color: Color::rgb(0.6, 0.5, 0.4),
                            font_size: 10.0,
                        },
                    ),
                    style: Style {
                        margin: UiRect {
                            bottom: Val::Px(5.0),
                            top: Val::Px(2.0),
                            ..Default::default()
                        },
                        // align_items: AlignItems::Center,
                        position: UiRect {
                            left: Val::Percent(-25.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    focus_policy: FocusPolicy::Pass,
                    ..Default::default()
                });
                p.spawn(ImageBundle {
                    style: Style {
                        size: Size::new(Val::Auto, Val::Percent(100.0)),
                        ..Default::default()
                    },
                    image: assets.gate_image(g).into(),
                    focus_policy: FocusPolicy::Pass,
                    ..Default::default()
                });
            });
        }
    });
}

fn spawn_gate(
    mut c: Commands,
    assets: Res<Assets>,
    buttons: Query<(&Interaction, &GlobalTransform, &Gate), Changed<Interaction>>,
) {
    for (button, pos, g) in buttons.iter() {
        match button {
            Interaction::Clicked => {
                // bevy::prelude::info!("spawning");
                c.spawn((
                    SpriteBundle {
                        texture: assets.gate_image(*g).clone(),
                        transform: Transform {
                            // scale: Vec3::splat(4.0),
                            translation: Vec3::new(10000.0, 10000.0, pos.translation().z),
                            ..Default::default()
                        },
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(110., 110.) / 2.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    UnPlaced(Vec2::splat(0.0)),
                    *g,
                    Collider::cuboid(110.0 / 4.0, 110.0 / 4.0),
                ));
            }
            _ => (),
        }
    }
}

fn unplace_gate(
    mut c: Commands,
    rapier_context: Res<RapierContext>,
    mou: Res<Input<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    windows: Res<Windows>,
    gates: Query<(&Transform, Entity), With<Gate>>,
) {
    let (camera, camera_transform) = q_camera.single();
    if mou.just_pressed(MouseButton::Left) {
        let wnd = windows.get_primary().unwrap();
        if let Some(p) = wnd.cursor_position() {
            // get the size of the window
            let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
            // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
            let ndc = (p / window_size) * 2.0 - Vec2::ONE;
            // matrix for undoing the projection and camera transform
            let ndc_to_world =
                camera_transform.compute_matrix() * camera.projection_matrix().inverse();
            // use it to convert ndc to world-space coordinates
            let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
            // reduce it to a 2D value
            let world_pos: Vec2 = world_pos.truncate();

            // rapier_context.intersections_with_point(world_pos, QueryFilter::default(), |e| {
            //     c.entity(e).insert(UnPlaced(
            //        world_pos - gates.get(e).unwrap().0.translation.truncate(), // BAD: found a bug in bevy. gates.get(e) gives wrong result for some reason
            //     ));
            //     bevy::prelude::info!("unplaced e: {:?} pos: {:?}", e, gates.get(e).unwrap().0.translation.truncate());
            //     bevy::prelude::info!("offset {:?}", world_pos - gates.get(e).unwrap().0.translation.truncate());
            //     bevy::prelude::info!("found_entity {:?}", gates.get(e).unwrap().1);
            //     false
            // });
            rapier_context.intersections_with_point(world_pos, QueryFilter::default(), |e| {
                c.entity(e).insert(UnPlaced(
                    world_pos
                        - gates
                            .iter()
                            .find(|k| k.1 == e)
                            .unwrap()
                            .0
                            .translation
                            .truncate(),
                ));
                false
            });
        }
    }
}

fn handle_unplaced(
    mut c: Commands,
    mut unplaced_gate: Query<(&mut Transform, Entity, &Gate, &UnPlaced)>,
    mou: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    // buttons: Query<(&Interaction, &Gate)>,
    palette: Query<&Interaction, With<GatePalette>>,
) {
    let (camera, camera_transform) = q_camera.single();
    if let Ok((mut pos, e, _g, upos)) = unplaced_gate.get_single_mut() {
        if mou.just_released(MouseButton::Left) {
            // try to place
            if palette.iter().any(|p| *p == Interaction::Hovered) {
                // if still in the button, just delete it
                c.entity(e).despawn();
                // bevy::prelude::info!("despawning");
            } else {
                c.entity(e).remove::<UnPlaced>();
                // bevy::prelude::info!("placed e: {e:?} pos: {:?}", pos.translation.truncate());
            }
        } else if mou.pressed(MouseButton::Left) {
            let wnd = windows.get_primary().unwrap();
            if let Some(p) = wnd.cursor_position() {
                // get the size of the window
                let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
                // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
                let ndc = (p / window_size) * 2.0 - Vec2::ONE;
                // matrix for undoing the projection and camera transform
                let ndc_to_world =
                    camera_transform.compute_matrix() * camera.projection_matrix().inverse();
                // use it to convert ndc to world-space coordinates
                let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
                // reduce it to a 2D value
                let world_pos: Vec2 = world_pos.truncate();

                let world_pos = world_pos - upos.0;
                // bevy::prelude::info!("upos: {}, e: {:?}", upos.0, e);
                pos.translation = Vec3::new(world_pos.x, world_pos.y, pos.translation.z);

                pos.translation /= 5.0;
                pos.translation = Vec3::new(
                    pos.translation.x.round(),
                    pos.translation.y.round(),
                    pos.translation.z,
                );
                pos.translation *= 5.0;
                // bevy::prelude::info!("{}", format!("{:#?}", pos.translation));
            }
        }
    }
}

// #[cfg(debug_assertions)]

#[derive(Resource)]
pub struct Assets {
    pub and_gate: Handle<Image>,
    pub or_gate: Handle<Image>,
    pub not_gate: Handle<Image>,
    pub font: Handle<Font>,
}

impl Assets {
    fn gate_image(&self, g: Gate) -> Handle<Image> {
        match g {
            Gate::And => self.and_gate.clone(),
            Gate::Or => self.or_gate.clone(),
            Gate::Not => self.not_gate.clone(),
        }
    }
}

#[derive(Copy, Clone, Debug, Component, PartialEq, Eq)]
pub enum Gate {
    And,
    Or,
    Not,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct UnPlaced(Vec2);

#[derive(Component)]
pub struct GatePalette;
