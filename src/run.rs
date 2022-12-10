use anyhow::Result;
use bevy::{
    prelude::{
        App, AssetServer, BuildChildren, ButtonBundle, Camera, Camera2dBundle, Changed, ClearColor,
        Color, Commands, Component, Entity, GlobalTransform, Handle, Image, ImageBundle,
        ImagePlugin, Input, MouseButton, PluginGroup, Query, Res, ResMut, Resource, TextBundle,
        Transform, Vec2, Vec3, With, Name,
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
use bevy_asset_loader::prelude::{AssetCollection, LoadingState, LoadingStateAppExt};
use bevy_inspector_egui::{bevy_egui::EguiSettings, WorldInspectorPlugin};
use bevy_rapier2d::prelude::{Collider, QueryFilter, RapierContext, RapierPhysicsPlugin};
use iyes_loopless::prelude::{AppLooplessStateExt, ConditionSet};

pub fn run(mut app: App) -> Result<()> {
    app.insert_resource(ClearColor(Color::rgb(0.25, 0.3, 0.25)))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "gatos".to_string(),
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
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Playing)
                .with_collection::<Assets>(),
        )
        .add_enter_system(GameState::Loading, spawn)
        .add_enter_system(GameState::Playing, spawn_ui)
        // .add_enter_system(GameState::Playing, create_wire_sprite) // ? temp
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Playing)
                .with_system(spawn_gate)
                .with_system(handle_unplaced)
                .with_system(unplace_gate)
                .with_system(spawn_wires)
                // .with_system(finalise_wire)
                .with_system(create_wire_sprite)
                .into(),
        )
        .add_plugin(WorldInspectorPlugin::new())
        .insert_resource(EguiSettings {
            scale_factor: 0.5,
            ..Default::default()
        })
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

        let world_pos = (world_pos / 5.0).round() * 5.0;
        let wire_bundle = (
            WireNode,
            TransformBundle {
                local: Transform {
                    translation: Vec3::new(world_pos.x, world_pos.y, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        if mou.just_pressed(MouseButton::Right) {
            let id = c
                .spawn(wire_bundle)
                .id();
            wire.nodes.push(id);
        } else if mou.just_released(MouseButton::Right) {
            let id = c
                .spawn(wire_bundle)
                .id();
            wire.nodes.push(id);

            let w = std::mem::replace(
                &mut *wire,
                Wire {
                    nodes: Default::default(),
                },
            );
            c.spawn((w, UnFinalised));
        }
    }
}

fn finalise_wire(
    mut c: Commands,
    q: Query<&Transform, With<WireNode>>,
    mut wires: Query<(&mut Wire, Entity), With<UnFinalised>>,
) {
    for (wire, e) in wires.iter_mut() {
        c.entity(e).remove::<UnFinalised>();

        wire.nodes.iter().cloned().for_each(|e| {
            c.entity(e).insert(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.4, 0.5, 0.4, 1.0),
                    custom_size: Some(Vec2::new(10., 10.) / 2.0),
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

/*
// might need https://bevy-cheatbook.github.io/assets/assetevent.html
fn create_wire_sprite(
    mut c: Commands,
    mut image_store: ResMut<bevy::prelude::Assets<Image>>, // https://bevy-cheatbook.github.io/assets/data.html
    q: Query<(&Transform, Entity), With<WireNode>>,
    mut wires: Query<(&mut Wire, Entity), With<UnFinalised>>,
) {
        // let mut img = image::ImageBuffer::from_fn(20, 20, |x, y| {
        //     if (x + y) % 2 == 0 {
        //         image::Rgba([0., 0., 0., 0.])
        //     } else {
        //         image::Rgba([1., 1., 1., 1.])
        //     }
        // });

    for (wire, e) in wires.iter_mut() {
        c.entity(e).remove::<UnFinalised>();

        let start = q.iter().find(|k| k.1 == wire.nodes[0]).unwrap().0.translation;
        let stop = q.iter().find(|k| k.1 == wire.nodes[1]).unwrap().0.translation;

        let left = ((gleft-gleft)/5.0).round();
        let right = ((gright-gleft)/5.0).round();

        /*
        left = v(start.x.min(stop.x), start.y.min(stop.y)) // vecu32 or somehitng
        right = v(..max.., ..max..)
        hflip = !(left.x == start.x)
        vflip = !(left.y == start.y)
        h = right.x-left.x
        v = right.y-left.y
        right -= left
        offset = left
        left -= left
        for x in 0..(h/2) {
            if hflip {
                img[0][h-1-i] = 1
            } else {
                img[0][i] = 1
            }
        }

        for x in 0..(h-h/2) {
            if hflip {
                img[v-1][h-i-(h/2+i)] = 1
            } else {
                img[v-1][h/2+i] = 1
            }
        }

        off = (start - stop)/2.0
        pos = (start/5.0).rount()*5.0 + off
        */
    
        let w = (right.x - left.x).abs() as _;
        let h = (right.y - left.y).abs() as _;
        let mut img = image::Rgba32FImage::new(w, h);
    
        for x in 1..1 {
            img.put_pixel(x as _, y, image::Rgba([0.4, 0.5, 0.4, 1.]));
        }
        
        // img.put_pixel(0, 0, image::Rgba([1., 0., 0., 1.]));
        // now create a bevy Image from img as DynamicImage and use in a sprite
        let img = Image::from_dynamic(img.into(), true);
        c.spawn((
            SpriteBundle {
                texture: image_store.add(img.into()),
                transform: Transform {
                    // scale: Vec3::splat(4.0),
                    translation: ((gleft/5.0).round()*5.0+(gright/5.0).round()*5.0)/2.0,
                    ..Default::default()
                },
                sprite: Sprite {
                    custom_size: Some(Vec2::new(w as f32 * 10., h as f32 * 10.) / 2.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            Gate::Not,
            Collider::cuboid(w as f32 * 10.0 / 4.0, h as f32 * 10.0 / 4.0),
            Name::from("wire"),
        ));
    }
}
*/

// might need https://bevy-cheatbook.github.io/assets/assetevent.html
fn create_wire_sprite(
    mut c: Commands,
    mut image_store: ResMut<bevy::prelude::Assets<Image>>, // https://bevy-cheatbook.github.io/assets/data.html
    q: Query<(&Transform, Entity), With<WireNode>>,
    mut wires: Query<(&mut Wire, Entity), With<UnFinalised>>,
) {
    for (mut wire, e) in wires.iter_mut() {
        let old_len = wire.nodes.len();
        // bevy::log::info!("{:?}", wire.nodes.iter().cloned().map(|e| q.get(e).unwrap().0.translation).collect::<Vec<_>>());

        wire.nodes = wire.nodes.iter().cloned().zip(wire.nodes.iter().cloned().skip(1))
        .map(|(a, b)| {
            let at = q.get(a).unwrap();
            let bt = q.get(b).unwrap();
            if at.0.translation.x as i64 == bt.0.translation.x as i64 || at.0.translation.y as i64 == bt.0.translation.y as i64 {
                vec![a].into_iter()
            } else {
                // dbg!(at.0.translation.x as i64 == bt.0.translation.x as i64, at.0.translation.y as i64 == bt.0.translation.y as i64);
                let mut new_x = (at.0.translation.x + bt.0.translation.x)/2.0;
                new_x = (new_x/5.0).round()*5.0;
                let ce = c.spawn((
                    WireNode,
                    TransformBundle {
                        local: Transform {
                            translation: Vec3::new(new_x, at.0.translation.y, 0.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },        
                )).id();
                let de = c.spawn((
                    WireNode,
                    TransformBundle {
                        local: Transform {
                            translation: Vec3::new(new_x, bt.0.translation.y, 0.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },        
                )).id();
                vec![a, ce, de].into_iter()
            }
        }).flatten().chain([wire.nodes.iter().cloned().rev().next().unwrap()]).collect();
        if wire.nodes.len() != old_len {
            continue;
        }
        // bevy::log::info!("{:?}", wire.nodes.iter().cloned().map(|e| q.get(e).unwrap().0.translation).collect::<Vec<_>>());
        c.entity(e).remove::<UnFinalised>();

        let (l, r) = wire.nodes.iter().cloned()
        .map(|e| q.get(e).unwrap().0.translation)
        // .map(|t| (t.truncate(), t.truncate()))
        .map(|t| (t, t))
        .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
        .unwrap();
    
        dbg!(l, r, (r-l)/5.0);
        let w = ((r.x - l.x)/5.0).round() as u32;
        let h = ((r.y - l.y)/5.0).round() as u32;
        if w*h == 0 {
            bevy::log::error!("wire size zero");
            continue;
        }
        let mut img = image::Rgba32FImage::new(w, h);
        let v = wire.nodes.iter().cloned()
        .map(|e| q.get(e).unwrap().0.translation)
        .map(|t| t-l)
        // .inspect(|t| {dbg!(&t);})
        .map(|t| (t/5.0).round())
        .map(|t| (t.x as u32, t.y as u32))
        // .inspect(|t| {dbg!(&t);})
        ;
        
        v.clone().zip(v.clone().skip(1))
        .for_each(|(t1, t2)| {
            if t1.0 == t2.0 {
                let x = t1.0;
                for y in t1.1.min(t2.1)..t1.1.max(t2.1) {
                    img.put_pixel(x, h-y-1, image::Rgba([0.4, 0.5, 0.4, 1.]));
                }
            } else if t1.1 == t2.1 {
                let y = t1.1.min(h-1);
                for x in t1.0.min(t2.0)..t1.0.max(t2.0) {
                    img.put_pixel(x, h-y-1, image::Rgba([0.4, 0.5, 0.4, 1.]));
                }
            } else {
                unreachable!();
            }
        });
    
        // img.put_pixel(0, 0, image::Rgba([1., 0., 0., 1.]));
        // now create a bevy Image from img as DynamicImage and use in a sprite
        let img = Image::from_dynamic(img.into(), true);
        c.spawn((
            SpriteBundle {
                texture: image_store.add(img.into()),
                transform: Transform {
                    // scale: Vec3::splat(4.0),
                    // off = (start - stop)/2.0
                    // pos = (start/5.0).rount()*5.0 + off            
                    // translation: ((gleft/5.0).round()*5.0+(gright/5.0).round()*5.0)/2.0,
                    // translation: ((l/5.0).round()*5.0+(r/5.0).round()*5.0)/2.0,
                    translation: (l/5.0).round()*5.0 + (r-l)/2.0,
                    ..Default::default()
                },
                sprite: Sprite {
                    custom_size: Some(Vec2::new(w as f32 * 10., h as f32 * 10.) / 2.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            Gate::Not,
            Collider::cuboid(w as f32 * 10.0 / 4.0, h as f32 * 10.0 / 4.0),
            Name::from("wire"),
        ));
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

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    Loading,
    Playing,
}

fn spawn(mut c: Commands) {
    c.spawn(Camera2dBundle::default());
    c.insert_resource(Wire { nodes: vec![] });
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
                            // left: Val::Percent(-25.0),
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
                bevy::prelude::info!("spawning");
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

            rapier_context.intersections_with_point(world_pos, QueryFilter::default(), |e| {
                c.entity(e).insert(UnPlaced(
                    world_pos
                        - gates
                            // .iter()
                            // .find(|k| k.1 == e)
                            .get(e)
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
            bevy::prelude::info!("just released");
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

#[derive(Resource, AssetCollection)]
pub struct Assets {
    #[asset(path = "sprites/and_gate.png")]
    pub and_gate: Handle<Image>,
    #[asset(path = "sprites/or_gate.png")]
    pub or_gate: Handle<Image>,
    #[asset(path = "sprites/not_gate.png")]
    pub not_gate: Handle<Image>,
    #[asset(path = "fonts/VarelaRound-Regular.ttf")]
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
