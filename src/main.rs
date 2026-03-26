use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};
use std::ops::{Deref, DerefMut};

mod minesweeper;
use minesweeper::{CellSnapshot, CellState, MinesweeperBoard, RevealOutcome};

const CELL_SIZE: f32 = 36.0;
const CELL_GAP: f32 = 2.0;
const STEP: f32 = CELL_SIZE + CELL_GAP;
// Shift the board down slightly to leave room for the UI bar at the top
const BOARD_Y_OFFSET: f32 = -30.0;

const COLOR_HIDDEN: Color = Color::srgb(0.55, 0.55, 0.60);
const COLOR_REVEALED: Color = Color::srgb(0.85, 0.85, 0.85);
const COLOR_MINE: Color = Color::srgb(0.80, 0.15, 0.15);
const COLOR_FLAG: Color = Color::srgb(0.85, 0.65, 0.10);
const COLOR_CHORD_HINT: Color = Color::srgb(0.75, 0.75, 0.80);

fn cell_base_color(cell: &CellSnapshot) -> Color {
    match cell.state {
        CellState::Flagged => COLOR_FLAG,
        CellState::Questioned => COLOR_HIDDEN,
        CellState::RevealedMine => COLOR_MINE,
        CellState::RevealedEmpty | CellState::RevealedNumber(_) => COLOR_REVEALED,
        CellState::Hidden => COLOR_HIDDEN,
    }
}

fn num_color(n: u8) -> Color {
    match n {
        1 => Color::srgb(0.1, 0.1, 0.9),
        2 => Color::srgb(0.0, 0.6, 0.0),
        3 => Color::srgb(0.9, 0.1, 0.1),
        4 => Color::srgb(0.0, 0.0, 0.6),
        5 => Color::srgb(0.6, 0.0, 0.0),
        6 => Color::srgb(0.0, 0.6, 0.6),
        7 => Color::srgb(0.0, 0.0, 0.0),
        _ => Color::srgb(0.5, 0.5, 0.5),
    }
}

// ── State ────────────────────────────────────────────────────────────────────

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum GameState {
    #[default]
    Playing,
    Won,
    Lost,
}

// ── Board resource ────────────────────────────────────────────────────────────

#[derive(Resource, Clone, Copy)]
struct BoardConfig {
    width: u16,
    height: u16,
    mine_count: usize,
}

#[derive(Resource)]
struct Board(MinesweeperBoard);

impl BoardConfig {
    fn new(width: u16, height: u16, mine_count: usize) -> Self {
        Self {
            width,
            height,
            mine_count,
        }
    }

    fn board_pixel_width(&self) -> f32 {
        self.width as f32 * STEP - CELL_GAP
    }

    fn board_pixel_height(&self) -> f32 {
        self.height as f32 * STEP - CELL_GAP
    }

    fn origin(&self) -> Vec2 {
        Vec2::new(
            -self.board_pixel_width() / 2.0 + CELL_SIZE / 2.0,
            -self.board_pixel_height() / 2.0 + CELL_SIZE / 2.0 + BOARD_Y_OFFSET,
        )
    }

    fn cell_world_pos(&self, x: usize, y: usize) -> Vec2 {
        let origin = self.origin();
        Vec2::new(origin.x + x as f32 * STEP, origin.y + y as f32 * STEP)
    }

    fn world_to_cell(&self, world: Vec2) -> Option<(usize, usize)> {
        let origin = self.origin();
        let cx = ((world.x - origin.x + STEP / 2.0) / STEP) as i32;
        let cy = ((world.y - origin.y + STEP / 2.0) / STEP) as i32;

        if cx < 0 || cy < 0 || cx >= self.width as i32 || cy >= self.height as i32 {
            return None;
        }

        Some((cx as usize, cy as usize))
    }

    fn window_resolution(&self) -> WindowResolution {
        WindowResolution::new(
            (self.board_pixel_width() + 80.0).max(650.0) as u32,
            (self.board_pixel_height() + 160.0).max(720.0) as u32,
        )
    }
}

impl Board {
    fn from_config(config: BoardConfig) -> Self {
        Self(MinesweeperBoard::new(
            config.width,
            config.height,
            config.mine_count,
        ))
    }
}

impl Deref for Board {
    type Target = MinesweeperBoard;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Board {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
struct CellMarker {
    x: usize,
    y: usize,
}

/// Marker for text overlays (numbers, flags, mine icons) spawned on the board.
#[derive(Component)]
struct CellText;

#[derive(Component)]
struct MineCounterText;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct ResetButton;

/// Set to true by the reset button; consumed by handle_reset.
#[derive(Resource, Default)]
struct ResetRequested(bool);

/// Cells currently lit up by the chord-hold highlight.
#[derive(Resource, Default)]
struct ChordHighlight(Vec<(usize, usize)>);

/// The revealed numbered cell currently being chord-held.
#[derive(Resource, Default)]
struct ChordAnchor(Option<(usize, usize)>);

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let board_config = BoardConfig::new(16, 16, 40);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minesweeper".into(),
                resolution: board_config.window_resolution(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.22, 0.22, 0.27)))
        .insert_resource(board_config)
        .init_state::<GameState>()
        .insert_resource(Board::from_config(board_config))
        .insert_resource(ResetRequested::default())
        .insert_resource(ChordHighlight::default())
        .insert_resource(ChordAnchor::default())
        .add_systems(Startup, (setup_camera, setup_board, setup_ui))
        .add_systems(Update, handle_cell_click)
        .add_systems(Update, handle_reset_button)
        .add_systems(Update, handle_reset.after(handle_reset_button))
        .add_systems(
            Update,
            update_cell_visuals
                .after(handle_cell_click)
                .after(handle_reset),
        )
        .add_systems(
            Update,
            // Runs after update_cell_visuals so highlight colors win
            update_chord_highlight
                .after(update_cell_visuals)
                .after(handle_reset),
        )
        .add_systems(
            Update,
            update_ui.after(handle_cell_click).after(handle_reset),
        )
        .run();
}

// ── Startup systems ───────────────────────────────────────────────────────────

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_board(mut commands: Commands, config: Res<BoardConfig>) {
    for x in 0..config.width as usize {
        for y in 0..config.height as usize {
            let pos = config.cell_world_pos(x, y);
            commands.spawn((
                Sprite {
                    color: COLOR_HIDDEN,
                    custom_size: Some(Vec2::splat(CELL_SIZE)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 0.0),
                CellMarker { x, y },
            ));
        }
    }
}

fn setup_ui(mut commands: Commands, board: Res<Board>) {
    // Top bar: mine counter | new-game button | status text
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(60.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.14, 0.18)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("Mines: {}", board.mine_count())),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                MineCounterText,
            ));

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(110.0),
                        height: Val::Px(38.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.6, 0.6, 0.65)),
                    BackgroundColor(Color::srgb(0.35, 0.35, 0.42)),
                    ResetButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("New Game"),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                StatusText,
            ));
        });
}

// ── Update systems ────────────────────────────────────────────────────────────

fn handle_cell_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    config: Res<BoardConfig>,
    mut board: ResMut<Board>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
    mut chord_anchor: ResMut<ChordAnchor>,
) {
    if *current_state.get() != GameState::Playing {
        return;
    }
    let left_pressed = mouse.just_pressed(MouseButton::Left);
    let right_pressed = mouse.just_pressed(MouseButton::Right);
    let left_released = mouse.just_released(MouseButton::Left);
    let right_released = mouse.just_released(MouseButton::Right);
    let right_held = mouse.pressed(MouseButton::Right);

    // If an active chord hold ends, resolve it from the stored anchor cell.
    let chord_trigger = chord_anchor.0.is_some() && (left_released || right_released);
    let left_only = left_pressed && !right_held;

    if !chord_trigger && !left_only && !right_pressed {
        return;
    }

    if chord_trigger {
        if let Some((anchor_x, anchor_y)) = chord_anchor.0.take() {
            if board.chord(anchor_x, anchor_y) {
                next_state.set(GameState::Lost);
            } else if board.check_win() {
                next_state.set(GameState::Won);
            }
        }
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor) else {
        return;
    };

    let Some((cx, cy)) = config.world_to_cell(world) else {
        return;
    };

    // Right press places/removes a flag on unrevealed cells, even while left is
    // held. Guarding on !revealed avoids a spurious DerefMut that would mark the
    // board as changed every frame and thrash update_cell_visuals / the hint.
    if right_pressed && !board.is_revealed(cx, cy) {
        board.toggle_flag(cx, cy);
        return;
    }

    // left_only: normal reveal (ignore clicks on already-revealed or flagged cells)
    match board.reveal_at(cx, cy) {
        RevealOutcome::Ignored => {}
        RevealOutcome::HitMine => next_state.set(GameState::Lost),
        RevealOutcome::Safe if board.check_win() => next_state.set(GameState::Won),
        RevealOutcome::Safe => {}
    }
}

fn update_cell_visuals(
    board: Res<Board>,
    config: Res<BoardConfig>,
    mut cell_q: Query<(&CellMarker, &mut Sprite)>,
    text_q: Query<Entity, With<CellText>>,
    mut commands: Commands,
) {
    if !board.is_changed() {
        return;
    }

    // Remove all previous text overlays
    for e in &text_q {
        commands.entity(e).despawn();
    }

    for (marker, mut sprite) in &mut cell_q {
        let cell = board.cell_snapshot(marker.x, marker.y);

        sprite.color = cell_base_color(&cell);

        let pos = config.cell_world_pos(marker.x, marker.y);

        match cell.state {
            CellState::Flagged => {
                commands.spawn((
                    Text2d::new("F"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 0.15, 0.15)),
                    Transform::from_xyz(pos.x, pos.y, 1.0),
                    CellText,
                ));
            }
            CellState::Questioned => {
                commands.spawn((
                    Text2d::new("?"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::BLACK),
                    Transform::from_xyz(pos.x, pos.y, 1.0),
                    CellText,
                ));
            }
            CellState::RevealedMine => {
                commands.spawn((
                    Text2d::new("X"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::BLACK),
                    Transform::from_xyz(pos.x, pos.y, 1.0),
                    CellText,
                ));
            }
            CellState::RevealedNumber(n) => {
                commands.spawn((
                    Text2d::new(n.to_string()),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(num_color(n)),
                    Transform::from_xyz(pos.x, pos.y, 1.0),
                    CellText,
                ));
            }
            CellState::Hidden | CellState::RevealedEmpty => {}
        }
    }
}

fn update_ui(
    board: Res<Board>,
    game_state: Res<State<GameState>>,
    mut counter_q: Query<&mut Text, (With<MineCounterText>, Without<StatusText>)>,
    mut status_q: Query<(&mut Text, &mut TextColor), (With<StatusText>, Without<MineCounterText>)>,
) {
    if !board.is_changed() && !game_state.is_changed() {
        return;
    }

    if let Ok(mut text) = counter_q.single_mut() {
        let remaining = board.mine_count() as i32 - board.flags_placed() as i32;
        text.0 = format!("Mines: {remaining}");
    }

    if let Ok((mut text, mut color)) = status_q.single_mut() {
        match game_state.get() {
            GameState::Playing => {
                text.0 = String::new();
                color.0 = Color::WHITE;
            }
            GameState::Won => {
                text.0 = "You Won!".to_string();
                color.0 = Color::srgb(0.2, 0.9, 0.2);
            }
            GameState::Lost => {
                text.0 = "Game Over".to_string();
                color.0 = Color::srgb(0.9, 0.2, 0.2);
            }
        }
    }
}

fn handle_reset_button(
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<ResetButton>)>,
    mut reset_requested: ResMut<ResetRequested>,
) {
    for interaction in &interaction_q {
        if *interaction == Interaction::Pressed {
            reset_requested.0 = true;
        }
    }
}

fn handle_reset(
    mut reset_requested: ResMut<ResetRequested>,
    config: Res<BoardConfig>,
    mut board: ResMut<Board>,
    mut next_state: ResMut<NextState<GameState>>,
    mut highlight: ResMut<ChordHighlight>,
    mut chord_anchor: ResMut<ChordAnchor>,
) {
    if !reset_requested.0 {
        return;
    }
    reset_requested.0 = false;
    *board = Board::from_config(*config);
    highlight.0.clear();
    chord_anchor.0 = None;
    next_state.set(GameState::Playing);
}

/// While both mouse buttons are held over a revealed numbered cell, tint the
/// unrevealed unflagged neighbors with the classic "pressed" chord preview.
/// Runs every frame after update_cell_visuals so the tint overrides base colors.
fn update_chord_highlight(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    config: Res<BoardConfig>,
    board: Res<Board>,
    mut highlight: ResMut<ChordHighlight>,
    mut chord_anchor: ResMut<ChordAnchor>,
    mut cell_q: Query<(&CellMarker, &mut Sprite)>,
) {
    let both_held = mouse.pressed(MouseButton::Left) && mouse.pressed(MouseButton::Right);

    // Compute which cells should be highlighted this frame
    let mut new_cells: Vec<(usize, usize)> = Vec::new();
    let mut new_anchor = None;
    'compute: {
        if !both_held {
            break 'compute;
        }
        let Ok(window) = windows.single() else {
            break 'compute;
        };
        let Ok((camera, cam_transform)) = camera_q.single() else {
            break 'compute;
        };
        let Some(cursor) = window.cursor_position() else {
            break 'compute;
        };
        let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor) else {
            break 'compute;
        };

        let Some((cx, cy)) = config.world_to_cell(world) else {
            break 'compute;
        };

        new_anchor = Some((cx, cy));
        new_cells = board.chord_hint_cells(cx, cy);
    }

    let prev_cells = std::mem::replace(&mut highlight.0, new_cells.clone());
    let prev_anchor = std::mem::replace(&mut chord_anchor.0, new_anchor);

    // Nothing changed and board didn't change → skip sprite work
    if prev_cells == new_cells && prev_anchor == chord_anchor.0 && !board.is_changed() {
        return;
    }

    for (marker, mut sprite) in &mut cell_q {
        let pos = (marker.x, marker.y);
        let is_new = new_cells.contains(&pos);
        let was_prev = prev_cells.contains(&pos);

        if is_new {
            sprite.color = COLOR_CHORD_HINT;
        } else if was_prev {
            // Restore to base color (update_cell_visuals may have already done
            // this if the board changed, but it's harmless to redo it)
            sprite.color = cell_base_color(&board.cell_snapshot(marker.x, marker.y));
        }
    }
}
