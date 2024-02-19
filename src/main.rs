#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(unused)]

use std::path::PathBuf;
use chute::ui;
use eframe::egui::{self, load::Bytes};
use eframe::glow;
mod chute;

use chute::geometry;
use chute::parachute;
use evalexpr::ContextWithMutableVariables;

#[macro_use]
extern crate uom;

extern crate evalexpr;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).


    


    let native_options = eframe::NativeOptions {
        initial_window_size: Some([400.0, 300.0].into()),
        min_window_size: Some([300.0, 220.0].into()),
        icon_data: eframe::IconData::try_from_png_bytes(include_bytes!("../assets/parachute.png")).ok(),
        depth_buffer: 16,
        ..Default::default()
    };
    
    eframe::run_native(
        &format!("OpenChute v{}", env!("CARGO_PKG_VERSION")),
        native_options,
        Box::new(|cc| Box::<ChuteUI>::default()),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions {
        depth_buffer: 16,
        ..Default::default()
    };

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::<ChuteUI>::default()),
            )
            .await
            .expect("failed to start eframe");
    });
}




#[derive(Default,PartialEq,Clone,Copy)]
enum Tab {
    #[default] Design,
    Geometry,
    Experiment,
}

#[derive(Default)]
pub struct State {
    selected_tab: Tab,
    use_imperial: bool,
    project_file: Option<PathBuf>,
    display_filename: Option<String>,
}


#[derive(Default)]
struct ChuteUI {
    dropped_files: Vec<egui::DroppedFile>,
    name: String,
    selected: u8,
    initialised: bool,

    state: State,

    undoer: egui::util::undoer::Undoer<parachute::ChuteDesigner>,
    update_mesh: bool,

    designer: parachute::ChuteDesigner,
    renderer_3d: ui::Widget3D,

    math_expression: String,
    context: evalexpr::HashMapContext,
}

impl ChuteUI {
    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        ui.menu_button("File", |ui| {
            if ui.button("✨ New...").clicked() {
                self.new_project();
                ui.close_menu();
            }
            if ui.button("📁 Open").clicked() {
                self.open_project_file();
                ui.close_menu();
            }
            ui.separator();
            //if ui.button("💾 Save design").clicked() {
            //    todo!();
            //    ui.close_menu();
            //}
            if ui.button("💾 Save design as").clicked() {
                self.save_project_file();
                
                ui.close_menu();
            }
            if ui.button("Export DXF").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("*", &["dxf"]).save_file() {
                    self.designer.export_dxf(path);   
                }
                ui.close_menu();
            }
            if ui.button("Export PDF").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("*", &["pdf"]).save_file() {
                    self.designer.export_pdf(path);
                }
                ui.close_menu();
            }
            //if ui.button("Export SVG").clicked() {
            //    todo!();
            //    ui.close_menu();
            //}
        });


        ui.menu_button("Settings", |ui| {
            let unit_text = if self.state.use_imperial { "Use SI units" } else { "Use imperial units" };

            if ui.button(unit_text).clicked() {
                self.state.use_imperial = !self.state.use_imperial;
            }
            egui::widgets::global_dark_light_mode_buttons(ui);
        });

        if self.initialised {
            ui.separator();
            let mut selected_anchor = self.state.selected_tab;
            for (name, tab) in self.get_tabs() {
                if ui
                    .selectable_label(selected_anchor == tab, name)
                    .clicked()
                {
                    selected_anchor = tab;
                }
            }
            self.state.selected_tab = selected_anchor;
        }

        if let Some(filename) = &self.state.display_filename {        
            let mut filename_text: String = "File: ".to_owned();
            filename_text.push_str(filename);
            ui.separator();
            ui.label(filename_text);
        }

    }

    fn get_tabs(&self) -> impl Iterator<Item = (&str, Tab)> {
        let vec = vec![
            (
                "Design",
                Tab::Design,
            ),
            (
                "Geometry",
                Tab::Geometry,
            ),
            (
                "Experiment",
                Tab::Experiment,
            ),
        ];

        vec.into_iter()
    }

    fn design_tab(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {

        self.designer.update_calculations();
        ui.columns(2, |columns| {
            let mut ui = &mut columns[0];
            /* 
            
            
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
    
            egui::ComboBox::from_label("Select one!")
                .selected_text(format!("{:?}", self.selected))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected, 0, "Elliptical");
                    ui.selectable_value(&mut self.selected, 1, "Annular");
                    ui.selectable_value(&mut self.selected, 2, "DGB");
                    ui.selectable_value(&mut self.selected, 3, "Custom");
                }
            );
    
            ui.separator();

            ui.text_edit_singleline(&mut self.math_expression);
        
            let result: Option<f64> = match evalexpr::eval_with_context(&self.math_expression, &self.context) {
                Ok(evalexpr::Value::Float(val)) => Some(val),
                Ok(evalexpr::Value::Int(val)) => Some(val as f64),
                _ => None,
            };
            
            //let result = evalexpr::eval_float(&self.math_expression).unwrap_or(0.0);
    
            ui.label(format!("Result {}", result.unwrap_or(0.0)));
            */
    
            self.designer.options_ui(ui, frame, self.state.use_imperial);
    
            // Only update when underlying data changes
            if self.update_mesh {
                self.update_mesh = false;
                self.renderer_3d.handle_triangle(ui, Some(self.designer.get_3d_data()));
            } else {
                self.renderer_3d.handle_triangle(ui, None);
            }
            

            self.designer.instructions_ui(ui, frame);

            ui = &mut columns[1];
            
            self.designer.draw_cross_section(ui, frame, None);
            self.designer.draw_gores(ui, frame, None);

        });



    }

    fn geometry_tab(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.designer.geometry_ui(ui, frame, self.state.use_imperial);
    }

    fn experiment_tab(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.designer.experiment_ui(ui, frame, self.state.use_imperial);
    }

    fn new_project(&mut self) {
        // Start from scratch
        self.initialised = true;
        self.update_mesh = true;
        self.state.project_file = None;
        self.designer = parachute::ChuteDesigner::default();
    }

    fn load_project_file(&mut self, path: PathBuf) {
        // Check validity
        println!("PATH: {:?}", path);
        if path.extension().unwrap_or_default().to_str() == Some("chute") {
            self.state.display_filename = Some(path.file_name().unwrap().to_str().unwrap().to_owned());
            self.state.project_file = Some(path.clone());
            self.designer = parachute::ChuteDesigner::from_json(&std::fs::read_to_string(path).unwrap());
            self.initialised = true;
            self.update_mesh = true;
        }
    }

    fn load_project_bytes(&mut self, bytes: Vec<u8>) {
        println!("Loaded bytes: {}", bytes.len());
        self.state.display_filename = Some(format!("Bytes {}", bytes.len()));

        if let Ok(des) = String::from_utf8(bytes) {
            self.designer = parachute::ChuteDesigner::from_json(&des);
        }
        
        self.initialised = true;
        self.update_mesh = true;
    }

    fn open_project_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = rfd::FileDialog::new().add_filter("*", &["chute"]).pick_file() {
            self.load_project_file(path);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let future = async {
            let file = rfd::AsyncFileDialog::new()
                    .add_filter("*", &["chute"])
                    .set_directory("/")
                    .pick_file()
                    .await;
                
                if let Some(file_data) = file {
                    let data = file_data.read().await;
                    println!("{:?}", data);
                }
                //self.load_project_bytes(data)
            };
            wasm_bindgen_futures::spawn_local(future);
        }
        
    }

    fn save_project_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = rfd::FileDialog::new().add_filter("*", &["chute"]).save_file() {
            std::fs::write(path.clone(), self.designer.to_json().unwrap());
            self.state.display_filename = Some(path.file_name().unwrap().to_str().unwrap().to_owned());
            self.state.project_file = Some(path);
        }

        #[cfg(target_arch = "wasm32")]
        {

        }
    }

}

impl eframe::App for ChuteUI {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        //ctx.set_pixels_per_point(1.3);
        egui::TopBottomPanel::top("app_top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = false;
                self.bar_contents(ui, frame);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {

            egui::ScrollArea::vertical().id_source("scrollfirst")
            .show(ui, |ui| {
                if !self.initialised {
                    // Show buttons for loading a file or creating a new project
                    ui.heading("Welcome to OpenChute (alpha version) :D");
                    ui.label("You can also drag a design file onto here");
                    ui.horizontal(|ui| {
                        if ui.button("Open parachute…").clicked() {
                            self.open_project_file();
                        };
                        if ui.button("New parachute…").clicked() {
                            self.new_project();
                        };
                    });
                } else {
                    match self.state.selected_tab {
                        Tab::Design => self.design_tab(ui, frame),
                        Tab::Geometry => self.geometry_tab(ui, frame),
                        Tab::Experiment => self.experiment_tab(ui, frame),
                    }
                }
            })
        });

        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let file = i.raw.dropped_files.first().unwrap();

                if let Some(path) = &file.path {
                    self.load_project_file(path.clone());
                } else {
                    // Loading bytes is secondary method
                    if let Some(bytes) = &file.bytes {
                        self.load_project_bytes(bytes.to_vec());
                    }
                }
            }
        });

        
        ctx.input_mut(|i| {
            // Handle undo stuff
            self.update_mesh = self.update_mesh || self.undoer.is_in_flux();

            self.undoer.feed_state(i.time, &self.designer);
            if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::MAC_CMD, egui::Key::Z)) ||
               i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z)) {
                if let Some(to_undo) = self.undoer.undo(&self.designer) {
                    self.designer = to_undo.clone();
                }
            }
        });
    }
}

/// Preview hovering files:
fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::*;
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}


#[cfg(test)]
mod tests {
    
}