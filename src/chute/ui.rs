#![allow(unused)]

use std::f32::consts::PI;
use std::ops::RangeInclusive;
use eframe::egui::{self, Widget};
use eframe::glow;

use uom::si::f32::*;
use uom::si::length;
use uom::{si, ConversionFactor};

// Slider for handling a length including unit conversions
pub fn length_slider<S,I>(ui: &mut egui::Ui, value_m: &mut f64, use_imperial: bool, range: RangeInclusive<f64>, si_unit: &S, imperial_unit: &I) -> egui::Response
where
S: uom::Conversion<f64> + si::Unit,
I: uom::Conversion<f64> + si::Unit
{    
    let unit_abbrev = if use_imperial { I::abbreviation() } else { S::abbreviation() };
    let conversion_factor = if use_imperial { imperial_unit.conversion().value() } else { si_unit.conversion().value() };;


    let mut value = *value_m / conversion_factor;
    let new_range = (range.start() / conversion_factor).round() ..= (range.end() / conversion_factor).round();


    let field = egui::Slider::new::<f64>(&mut value, new_range)
                            .text(format!("[{}]", unit_abbrev))
                            .clamp_to_range(false);
    
    let res = ui.add(field)
                          .context_menu(|ui| {
                            if ui.button("Reset value").clicked() {
                                ui.close_menu();
                                todo!();
                            }
                        });

    *value_m = (value * conversion_factor).max(0.0).min(10000.0); // Absolute limits

    res
}

// Slider for handling a length including unit conversions
pub fn length_slider_no_limit<S,I>(ui: &mut egui::Ui, value_m: &mut f64, use_imperial: bool, range: RangeInclusive<f64>, si_unit: &S, imperial_unit: &I) -> egui::Response
where
S: uom::Conversion<f64> + si::Unit,
I: uom::Conversion<f64> + si::Unit
{    
    let unit_abbrev = if use_imperial { I::abbreviation() } else { S::abbreviation() };
    let conversion_factor = if use_imperial { imperial_unit.conversion().value() } else { si_unit.conversion().value() };;


    let mut value = *value_m / conversion_factor;
    let new_range = (range.start() / conversion_factor).round() ..= (range.end() / conversion_factor).round();


    let field = egui::Slider::new::<f64>(&mut value, new_range)
                            .text(format!("[{}]", unit_abbrev))
                            .clamp_to_range(false);
    
    let res = ui.add(field)
                          .context_menu(|ui| {
                            if ui.button("Reset value").clicked() {
                                ui.close_menu();
                                todo!();
                            }
                        });
    res
}

pub fn integer_edit_field(ui: &mut egui::Ui, value: &mut u16) -> egui::Response {
    let field = egui::Slider::new::<u16>(value, 4..=24)
                            .text("[]")
                            .clamp_to_range(false);
    
    let res = ui.add(field)
                          .context_menu(|ui| {
                            if ui.button("Reset value").clicked() {
                                ui.close_menu();
                            }
                        });

    res
}

pub fn delete_move_buttons(ui: &mut egui::Ui, to_delete: &mut Option<usize>, to_move: &mut Option<(usize, bool)>, idx: usize, num_parameters: usize) {
    if ui.button("❌").on_hover_text("Delete").clicked() {
        *to_delete = Some(idx);
    }
    if ui.add_enabled(idx != 0, egui::Button::new("⬆")).on_hover_text("Move up").clicked() {
        *to_move = Some((idx, true));
    }
    if ui.add_enabled(idx < num_parameters - 1, egui::Button::new("⬇")).on_hover_text("Move down").clicked() {
        *to_move = Some((idx, false));
    };
}

// Linear dimension. metric=m, imperial=ft.
// Always stored as m in backend
pub fn dimension_field(ui: &mut egui::Ui, value_metric: &mut f64, use_imperial: bool, range: RangeInclusive<f64>) -> egui::Response {

    let mut value = if use_imperial { *value_metric / 0.3048 } else { *value_metric };
    let new_range = if use_imperial { range.start() / 0.3048 ..= (range.end() / 0.3048).round() } else { range };


    ui.label("Diameter:").on_hover_text("Number of parachute gores. Typically between 6 and 24");
    let field = egui::Slider::new::<f64>(&mut value, new_range)
                            .text(if use_imperial { "[ft]" } else { "[m]" })
                            .clamp_to_range(false);

    let res = ui.add(field)
                          .context_menu(|ui| {
                            if ui.button("Reset value").clicked() {
                                ui.close_menu();
                            }
                        });

    *value_metric = (if use_imperial { value * 0.3048 } else { value }).max(0.0);

    res
}

pub fn number_edit_field(ui: &mut egui::Ui, value: &mut f64) -> egui::Response {
    let mut tmp_value = format!("{}", value);

    let res = ui.add(egui::TextEdit::singleline(&mut tmp_value).desired_width(10.0).clip_text(false));

    if let Ok(result) = tmp_value.parse() {
        *value = result;
    }
    res
}

#[derive(Default)]
pub struct Widget3D {
    angle_x: f32,
    angle_y: f32,
}

impl Widget3D {
    pub fn handle_triangle(&mut self, ui: &mut egui::Ui, meshes: Option<Vec<CpuMesh>>) {
    
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::Vec2::splat(512.0), egui::Sense::drag());

            let angle_delta = (response.drag_delta().x, response.drag_delta().y);
            self.angle_x += angle_delta.0 * 0.01;
            self.angle_y += angle_delta.1 * 0.01;

            // Clone locals so we can move them into the paint callback:
            let angle = (self.angle_x, self.angle_y);

            let callback = egui::PaintCallback {
                rect,
                callback: std::sync::Arc::new(egui_glow::CallbackFn::new(
                    move |info, painter| {
                        with_three_d(painter.gl(), |three_d| {
                            three_d.frame(
                                FrameInput::new(&three_d.context, &info, painter),
                                angle,
                                angle_delta,
                                meshes.clone()
                            );
                        });
                    },
                )),
            };
            ui.painter().add(callback);
        });
        ui.label("Drag to rotate!");

    }   
}


/* 3D stuff from here */

/// We get a [`glow::Context`] from `eframe` and we want to construct a [`ThreeDApp`].
///
/// Sadly we can't just create a [`ThreeDApp`] in [`MyApp::new`] and pass it
/// to the [`egui::PaintCallback`] because [`glow::Context`] isn't `Send+Sync` on web, which
/// [`egui::PaintCallback`] needs. If you do not target web, then you can construct the [`ThreeDApp`] in [`MyApp::new`].
pub fn with_three_d<R>(gl: &std::sync::Arc<glow::Context>, f: impl FnOnce(&mut ThreeDApp) -> R) -> R {
    use std::cell::RefCell;
    thread_local! {
        pub static THREE_D: RefCell<Option<ThreeDApp>> = RefCell::new(None);
    }

    THREE_D.with(|three_d| {
        let mut three_d = three_d.borrow_mut();
        let three_d = three_d.get_or_insert_with(|| ThreeDApp::new(gl.clone()));
        f(three_d)
    })
}

pub fn rgb_to_srgba(rgb: &[f32; 3]) -> three_d::Srgba {
    three_d::Srgba::new((rgb[0] * 255.0) as u8, (rgb[1] * 255.0) as u8, (rgb[2] * 255.0) as u8, 255)
}

///
/// Translates from egui input to three-d input
///
pub struct FrameInput<'a> {
    screen: three_d::RenderTarget<'a>,
    viewport: three_d::Viewport,
    scissor_box: three_d::ScissorBox,
}

impl FrameInput<'_> {
    pub fn new(
        context: &three_d::Context,
        info: &egui::PaintCallbackInfo,
        painter: &egui_glow::Painter,
    ) -> Self {
        use three_d::*;

        // Disable sRGB textures for three-d
        #[cfg(not(target_arch = "wasm32"))]
        #[allow(unsafe_code)]
        unsafe {
            use glow::HasContext as _;
            context.disable(glow::FRAMEBUFFER_SRGB);
        }

        // Constructs a screen render target to render the final image to
        let screen = painter.intermediate_fbo().map_or_else(
            || {
                RenderTarget::screen(
                    context,
                    info.viewport.width() as u32,
                    info.viewport.height() as u32,
                )
            },
            |fbo| {
                RenderTarget::from_framebuffer(
                    context,
                    info.viewport.width() as u32,
                    info.viewport.height() as u32,
                    fbo,
                )
            },
        );

        // Set where to paint
        let viewport = info.viewport_in_pixels();
        let viewport = Viewport {
            x: viewport.left_px.round() as _,
            y: viewport.from_bottom_px.round() as _,
            width: viewport.width_px.round() as _,
            height: viewport.height_px.round() as _,
        };

        // Respect the egui clip region (e.g. if we are inside an `egui::ScrollArea`).
        let clip_rect = info.clip_rect_in_pixels();
        let scissor_box = ScissorBox {
            x: clip_rect.left_px.round() as _,
            y: clip_rect.from_bottom_px.round() as _,
            width: clip_rect.width_px.round() as _,
            height: clip_rect.height_px.round() as _,
        };
        Self {
            screen,
            scissor_box,
            viewport,
        }
    }
}



///
/// Based on the `three-d` [Triangle example](https://github.com/asny/three-d/blob/master/examples/triangle/src/main.rs).
/// This is where you'll need to customize
///
use three_d::*;

use super::geometry;
pub struct ThreeDApp {
    context: Context,
    camera: Camera,
    model: Gm<Mesh, PhysicalMaterial>,
    light0: DirectionalLight,
    light1: DirectionalLight,
    control: OrbitControl,
}

impl ThreeDApp {
    pub fn new(gl: std::sync::Arc<glow::Context>) -> Self {
        let context = Context::from_gl_context(gl).unwrap();
        // Create a camera
        let camera = Camera::new_perspective(
            Viewport::new_at_origo(1, 1),
            vec3(0.0, 0.0, 2.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            degrees(45.0),
            0.1,
            10.0,
        );

        // Create a CPU-side mesh consisting of a single colored triangle
        let positions = vec![
            vec3(0.0, 0.0, 0.0),  // bottom right
            vec3(1.0, 0.0, 0.0), // bottom left
            vec3(0.0, 1.0, 0.0),   // top
            vec3(0.0, 0.0, 1.0),
        ];
        let colors = vec![
            Srgba::new(255, 0, 0, 255), // bottom right
            Srgba::new(0, 255, 0, 255), // bottom left
            Srgba::new(0, 0, 255, 255), // top
            Srgba::new(0, 100, 255, 255),
        ];
        let indices = vec![1, 0, 2, 2, 0, 3, 3, 0, 1];

        /*
        let mut cpu_mesh = CpuMesh {
            positions: Positions::F32(positions),
            colors: Some(colors),
            indices: Indices::U32(indices),
            ..Default::default()
        };

        let mut chute_cross = vec![];

        // Make cross section. Aligned with x y axis (y axis is up)
        for ang in 0..80 {
            chute_cross.push(vec3((ang as f32 * PI/180.0).cos(),(ang as f32 * PI/180.0).sin() * 0.7, 0.0));
        }
        let num_points_per_gore = chute_cross.len();

        let mut chute_coords: Vec<Vector3<f32>> = vec![];

        // revolve around y axis
        let num_gores = 16;
        for idx in 0..num_gores {
            let z_angle = idx as f32 / (num_gores as f32) * PI * 2.0;
            let cos_sin = (z_angle.cos(), z_angle.sin());
            // Append twice to allow per gore coloring
            chute_coords.append(&mut chute_cross.iter().map(|&pt| vec3(pt.x * cos_sin.0, pt.y, pt.x * cos_sin.1) ).collect::<Vec<_>>());
            chute_coords.append(&mut chute_cross.iter().map(|&pt| vec3(pt.x * cos_sin.0, pt.y, pt.x * cos_sin.1) ).collect::<Vec<_>>());
        }

        let mut triangle_indices: Vec<u32> = vec![];
        // Generate triangles. 
        for gore_idx in 0..num_gores {
            // Each gore
            for point_idx in 0..(num_points_per_gore-1) {
                // Each point on the left of that gore

                // Get four points forming a square
                let pt_left0 = ((gore_idx*2+1) * num_points_per_gore + point_idx) as u32;
                let pt_left1 = pt_left0 + 1;
                let pt_right0 = ((((gore_idx*2+1) + 1) % (num_gores * 2)) * num_points_per_gore + point_idx) as u32;
                let pt_right1 = pt_right0 + 1;
                
                // Two triangles forming a square
                // Order counterclockwise
                triangle_indices.append(&mut vec![pt_left0, pt_right0, pt_left1, pt_right0, pt_right1, pt_left1]);
            }            
        }

        let mut cpu_mesh = CpuMesh {
            positions: Positions::F32(chute_coords),
            indices: Indices::U32(triangle_indices),
            ..Default::default()
        };
        

        //let mut cpu_mesh = CpuMesh::cube();

        let mut new_colors: Vec<Srgba> = vec![];

        for j in 0..num_points_per_gore {
            new_colors.push(Srgba::new(255, 0, 0, 255));
        }

        for i in 0..num_gores {
            for j in 0..num_points_per_gore {
                new_colors.push(Srgba::new((i % 2) as u8 * 255, 0, 0, 255));
                new_colors.push(Srgba::new((i % 2) as u8 * 255, 0, 0, 255));
            }
        }

        //for idx in 0..cpu_mesh.vertex_count() {
        //    new_colors.push(Srgba::new((((idx % (5 * num_points_per_gore))/num_points_per_gore * 255) % 256) as u8 , 0 as u8, 0 as u8, 255))
        //}
        cpu_mesh.colors = Some(new_colors);

        cpu_mesh.compute_normals();
         */
        let mut des = crate::parachute::ChuteDesigner::default();
        des.update_calculations();
        let mut cpu_mesh = crate::parachute::ChuteDesigner::default().get_3d_data()[0].clone();

        // Construct a model, with a default color material, thereby transferring the mesh data to the GPU
        let model_old = Gm::new(Mesh::new(&context, &cpu_mesh), PhysicalMaterial::new_opaque(&context, &CpuMaterial {
            albedo: Srgba::WHITE,
            ..Default::default()
        }));

        let mut model = Gm::new(
            Mesh::new(&context, &CpuMesh::cube()),
            PhysicalMaterial::new_opaque(
                &context,
                &CpuMaterial {
                    albedo: Srgba {
                        r: 0,
                        g: 0,
                        b: 255,
                        a: 100,
                    },
                    ..Default::default()
                },
            ),
        );

        let light0 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, -0.5, -0.5));
        let light1 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, 0.5, 0.5));


        let mut control = OrbitControl::new(*camera.target(), 1.0, 100.0);

        Self {
            context,
            camera,
            model: model_old,
            light0,
            light1,
            control,
        }
    }

    pub fn frame(&mut self, frame_input: FrameInput<'_>, angle: (f32, f32), angle_delta: (f32, f32), meshes: Option<Vec<CpuMesh>>) -> Option<glow::Framebuffer> {
        // Ensure the viewport matches the current window viewport which changes if the window is resized
        self.camera.set_viewport(frame_input.viewport);
        self.camera.rotate_around_with_fixed_up(&vec3(0.0, 0.0, 0.0), angle_delta.0 * 0.1, angle_delta.1 * 0.1);
        
        // Set the current transformation of the triangle
        //self.model
        //    .set_transformation(Mat4::from_angle_x(radians(angle.1)) * Mat4::from_angle_y(radians(angle.0)));

        // Update model
        if let Some(meshes) = meshes {
            if let Some(mesh) = meshes.get(0) {
                self.model = Gm::new(Mesh::new(&self.context, mesh), PhysicalMaterial::new_opaque(&self.context, &CpuMaterial {
                    albedo: Srgba::WHITE,
                    ..Default::default()
                }));
            }
        } 

        // Get the screen render target to be able to render something on the screen
        frame_input
            .screen
            // Clear the color and depth of the screen render target
            .clear_partially(frame_input.scissor_box, ClearState::depth(1.0))
            // Render the triangle with the color material which uses the per vertex colors defined at construction
            .render_partially(frame_input.scissor_box, &self.camera, &[&self.model], &[&self.light0, &self.light1]);

        frame_input.screen.into_framebuffer() // Take back the screen fbo, we will continue to use it.
    }
}


