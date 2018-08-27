//! The map editor proper, with a GUI and everything.

extern crate glutin;
#[macro_use] extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
#[macro_use] extern crate imgui;
extern crate imgui_gfx_renderer;
extern crate lodepng;
extern crate ndarray;

extern crate dreammaker as dm;
extern crate dmm_tools;

mod support;
mod dmi;
mod map_renderer;

use std::sync::mpsc;

use imgui::*;

use dm::objtree::{ObjectTree, TypeRef};
use dmm_tools::dmm::Map;

use glutin::VirtualKeyCode as Key;
use gfx_device_gl::{Factory, Resources, CommandBuffer};
type Encoder = gfx::Encoder<Resources, CommandBuffer>;
type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;
type RenderTargetView = gfx::handle::RenderTargetView<Resources, ColorFormat>;
type Texture = gfx::handle::ShaderResourceView<Resources, [f32; 4]>;

fn main() {
    support::run("SpacemanDMM".to_owned(), [0.25, 0.25, 0.5, 1.0]);
}

pub struct EditorScene {
    map_renderer: map_renderer::MapRenderer,
    objtree: Option<ObjectTree>,

    maps: Vec<EditorMap>,
    map_current: usize,
    z_current: usize,

    objtree_rx: mpsc::Receiver<ObjectTree>,
    dmm_rx: mpsc::Receiver<Map>,

    ui_lock_windows: bool,
    ui_style_editor: bool,
    ui_imgui_metrics: bool,
    ui_debug: bool,
}

impl EditorScene {
    fn new(factory: &mut Factory, view: &RenderTargetView) -> Self {
        let (objtree_tx, objtree_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let context = dm::Context::default();
            let env = dm::detect_environment("tgstation.dme")
                .expect("error detecting .dme")
                .expect("no .dme found");
            let objtree = context.parse_environment(&env)
                .expect("i/o error opening .dme");
            let _ = objtree_tx.send(objtree);
        });

        let (dmm_tx, dmm_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let map = Map::from_file("_maps/map_files/debug/runtimestation.dmm".as_ref())
                .expect("error loading .dmm");
            let _ = dmm_tx.send(map);
        });

        EditorScene {
            map_renderer: map_renderer::MapRenderer::new(factory, view),
            objtree: None,
            maps: Vec::new(),
            map_current: 0,
            z_current: 0,

            objtree_rx,
            dmm_rx,

            ui_lock_windows: true,
            ui_style_editor: false,
            ui_imgui_metrics: false,
            ui_debug: true,
        }
    }

    fn mouse_wheel(&mut self, ctrl: bool, shift: bool, _alt: bool, _x: f32, y: f32) {
        let axis = if ctrl { 0 } else { 1 };
        let mul = if shift { 8.0 } else { 1.0 };

        self.map_renderer.center[axis] += 4.0 * 32.0 * mul * y / self.map_renderer.zoom;
    }

    fn chord(&mut self, _ctrl: bool, _shift: bool, _alt: bool, _key: Key) {
    }

    fn render(&mut self, factory: &mut Factory, encoder: &mut Encoder, view: &RenderTargetView) {
        if let Ok(objtree) = self.objtree_rx.try_recv() {
            self.map_renderer.icons.clear();
            if let Some(map) = self.maps.get(self.map_current) {
                self.map_renderer.prepare(factory, &objtree, &map.dmm, map.dmm.z_level(self.z_current));
            }
            self.objtree = Some(objtree);
        }
        if self.maps.is_empty() {
            if let Ok(dmm) = self.dmm_rx.try_recv() {
                if let Some(objtree) = self.objtree.as_ref() {
                    self.map_renderer.prepare(factory, &objtree, &dmm, dmm.z_level(self.z_current));
                }
                self.map_current = self.maps.len();
                self.maps.push(EditorMap { dmm });
            }
        }

        self.map_renderer.paint(factory, encoder, view);
    }

    fn run_ui(&mut self, ui: &Ui) -> bool {
        let mut continue_running = true;
        let mut window_positions_cond = match self.ui_lock_windows {
            false => ImGuiCond::FirstUseEver,
            true => ImGuiCond::Always,
        };

        ui.main_menu_bar(|| {
            ui.menu(im_str!("File")).build(|| {
                ui.menu_item(im_str!("Open Environment"))
                    .shortcut(im_str!("Ctrl+Shift+O"))
                    .enabled(false)
                    .build();
                ui.menu(im_str!("Recent Environments")).enabled(false).build(|| {
                    // TODO
                });
                ui.separator();
                ui.menu_item(im_str!("New"))
                    .shortcut(im_str!("Ctrl+N"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Open"))
                    .shortcut(im_str!("Ctrl+O"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Close"))
                    .shortcut(im_str!("Ctrl+W"))
                    .enabled(false)
                    .build();
                ui.separator();
                ui.menu_item(im_str!("Save"))
                    .shortcut(im_str!("Ctrl+S"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Save As"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Save Copy As"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Save All"))
                    .shortcut(im_str!("Ctrl+Shift+S"))
                    .enabled(false)
                    .build();
                ui.separator();
                if ui.menu_item(im_str!("Exit"))
                    .shortcut(im_str!("Alt+F4"))
                    .build()
                {
                    continue_running = false;
                }
            });
            ui.menu(im_str!("Edit")).build(|| {
                ui.menu_item(im_str!("Undo"))
                    .shortcut(im_str!("Ctrl+Z"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Redo"))
                    .shortcut(im_str!("Ctrl+Shift+Z"))
                    .enabled(false)
                    .build();
                ui.separator();
                ui.menu_item(im_str!("Cut"))
                    .shortcut(im_str!("Ctrl+X"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Copy"))
                    .shortcut(im_str!("Ctrl+C"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Paste"))
                    .shortcut(im_str!("Ctrl+V"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Delete"))
                    .shortcut(im_str!("Del"))
                    .enabled(false)
                    .build();
                ui.separator();
                ui.menu_item(im_str!("Select All"))
                    .shortcut(im_str!("Ctrl+A"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Select None"))
                    .shortcut(im_str!("Ctrl+Shift+A"))
                    .enabled(false)
                    .build();
            });
            ui.menu(im_str!("Zoom")).build(|| {
                for &zoom in [0.5, 1.0, 2.0, 4.0].iter() {
                    let mut selected = self.map_renderer.zoom == zoom;
                    if ui.menu_item(im_str!("{}%", 100.0 * zoom)).selected(&mut selected).build() {
                        self.map_renderer.zoom = zoom;
                    }
                }
            });
            ui.menu(im_str!("Window")).build(|| {
                ui.menu_item(im_str!("Lock positions"))
                    .selected(&mut self.ui_lock_windows)
                    .build();
                if ui.menu_item(im_str!("Reset positions")).enabled(!self.ui_lock_windows).build() {
                    window_positions_cond = ImGuiCond::Always;
                }
                ui.separator();
                ui.menu_item(im_str!("Debug Window"))
                    .selected(&mut self.ui_debug)
                    .build();
                ui.menu_item(im_str!("Style Editor"))
                    .selected(&mut self.ui_style_editor)
                    .build();
                ui.menu_item(im_str!("ImGui Metrics"))
                    .selected(&mut self.ui_imgui_metrics)
                    .build();
            });
            ui.menu(im_str!("Help")).build(|| {
                ui.menu_item(im_str!("About SpacemanDMM"))
                    .enabled(false)
                    .build();
                ui.menu_item(im_str!("Open-source licenses"))
                    .enabled(false)
                    .build();
                ui.menu(im_str!("ImGui help")).build(|| {
                    ui.show_user_guide();
                });
            });
        });
        if self.ui_style_editor {
            ui.window(im_str!("Style Editor"))
                .opened(&mut self.ui_style_editor)
                .build(|| ui.show_default_style_editor());
        }
        if self.ui_imgui_metrics {
            ui.show_metrics_window(&mut self.ui_imgui_metrics);
        }

        ui.window(im_str!("Object Tree"))
            .position((10.0, 30.0), window_positions_cond)
            .movable(!self.ui_lock_windows)
            .size((300.0, 600.0), ImGuiCond::FirstUseEver)
            .build(|| {
                if let Some(objtree) = self.objtree.as_ref() {
                    let root = objtree.root();
                    root_node(ui, root, "area");
                    root_node(ui, root, "turf");
                    root_node(ui, root, "obj");
                    root_node(ui, root, "mob");
                } else {
                    ui.text(im_str!("The object tree is loading..."));
                }
            });

        ui.window(im_str!("Maps"))
            .position((ui.frame_size().logical_size.0 as f32 - 310.0, 30.0), window_positions_cond)
            .movable(!self.ui_lock_windows)
            .size((300.0, 200.0), window_positions_cond)
            .resizable(!self.ui_lock_windows)
            .build(|| {
                for (map_idx, map) in self.maps.iter().enumerate() {
                    if ui.collapsing_header(im_str!("runtimestation.dmm")).default_open(true).build() {
                        ui.text(im_str!("keys: {} / length: {}",
                            map.dmm.dictionary.len(),
                            map.dmm.key_length));
                        for z in 0..map.dmm.dim_z() {
                            if ui.small_button(im_str!("z = {}", z + 1)) {
                                self.map_current = map_idx;
                                self.z_current = z;
                            }
                        }
                    }
                }
            });

        if self.ui_debug {
            let mut ui_debug = self.ui_debug;
            ui.window(im_str!("Debug")).opened(&mut ui_debug).build(|| {
                ui.text(im_str!("maps[{}], i = {}, z = {}", self.maps.len(), self.map_current, self.z_current));
                ui.text(im_str!("zoom = {}, center = {:?}", self.map_renderer.zoom, self.map_renderer.center));
                ui.text(im_str!("icons[{}], draw_calls[{}], atoms[{}]",
                    self.map_renderer.icons.len(),
                    self.map_renderer.draw_calls(),
                    self.map_renderer.last_atoms));
                ui.text(im_str!("last render: {}s", self.map_renderer.last_duration));
            });
            self.ui_debug = ui_debug;
        }

        continue_running
    }
}

struct EditorMap {
    dmm: Map,
}

fn root_node(ui: &Ui, ty: TypeRef, name: &str) {
    if let Some(child) = ty.child(name) {
        tree_node(ui, child);
    }
}

fn tree_node(ui: &Ui, ty: TypeRef) {
    let mut children = ty.children();
    if children.is_empty() {
        ui.tree_node(im_str!("{}", ty.name)).leaf(true).build(||{});
    } else {
        children.sort_by_key(|t| &t.get().name);
        ui.tree_node(im_str!("{}", ty.name)).build(|| {
            for child in children {
                tree_node(ui, child);
            }
        });
    }
}
